//! Quiz puzzle - Oruo the Omniscient asks 3 sequential SkyBlock trivia questions, 3 buttons
//! each (1 correct, 2 wrong). A wrong answer fails the room immediately; 3/3 correct completes
//! it. Researched, not guessed - see memory `project-quiz-puzzle-research` for the full writeup:
//!
//! - Hypixel Wiki: floor 4+, a statue asks 3 questions in sequence, wrong = instant fail.
//!   Reward is a direct "Blessing of Time" buff - not a lootable chest, 0 secrets in this room.
//! - A real captured player run (this project's own client-side chat/sound log) confirmed the
//!   exact chat lines, ~2s stage-transition pacing, and sound pair used below. The captured run
//!   never tested a wrong answer or a slow response, so there is no per-question player-facing
//!   timeout implemented here - only the confirmed scripted pacing between stages.
//! - The answer bank (33 questions) is Odin's (odtheking/Odin) real `quizAnswers.json`, which
//!   only stores the correct answer(s) per question, not wrong options - the wrong options below
//!   are original content authoring (same-category distractors), not scraped from anywhere.
//! - Room layout decoded from this project's own capture of this exact room
//!   (`room_data/rooms/18,quiz,-60,-600.json`): 3 pedestals in a triangle, each a Double Stone
//!   Slab with 4 real Stone Buttons (one per side) - 12 total, grouped 4-per-answer-index, any
//!   of the 4 around a pedestal counts as clicking that answer. No sign/hologram block exists
//!   anywhere in the capture, and the real captured run confirms the options also arrive as
//!   formatted chat, so this implementation is chat-only, matching Odin's own solver (which
//!   never spawns a hologram or tracks a statue entity either).
//!
//! Room-entry (not room-load) triggered, same idiom `three_weirdos::setup` uses - see
//! `Dungeon::tick`'s `rooms_just_entered` hook and `Room::quiz_started`. The timed sequence
//! (intro lines, inter-question pacing) is driven by chained `Server::schedule` one-shot
//! closures, the same technique `tic_tac_toe.rs::drop_wall` uses for its own delayed step,
//! rather than a per-tick state machine - there's nothing here that needs polling every tick,
//! only a chain of "wait N ticks, then do the next thing" steps.

use crate::dungeon::room::room::Room;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::entity::entity::{EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use crate::server::server::Server;
use crate::utils::seeded_rng::seeded_rng;
use rand::prelude::{IndexedRandom, SliceRandom};
use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Real, `[STATUE]`-prefixed intro speech - 4 lines, ~2s apart, before Question #1 appears.
const INTRO_LINES: [&str; 4] = [
    "I am Oruo the Omniscient. I have lived many lives. I have learned all there is to know.",
    "Though I sit stationary in this prison that is The Catacombs, my knowledge knows no bounds.",
    "Prove your knowledge by answering 3 questions and I shall reward you in ways that transcend time!",
    "Answer incorrectly, and your moment of ineptitude will live on for generations.",
];

/// Real scripted stage-transition pacing (confirmed from the captured run) - ~2s between any
/// two chat-driven stages (correct-message -> countdown line -> next question).
const STAGE_GAP_TICKS: u32 = 40;
/// Real gap between the final "answered the final question correctly!" message and the reward
/// flavor line.
const COMPLETION_GAP_TICKS: u32 = 20;
/// ~150-200ms real gap between the button-click sound and the "correct" level-up chime.
const CORRECT_CHIME_DELAY_TICKS: u32 = 4;

/// Room-relative pedestal centers, pre-rotation - decoded from this room's own captured block
/// data (see module doc comment). A triangle, all at `y=70`.
const PEDESTALS: [BlockPos; 3] = [
    BlockPos { x: 20, y: 70, z: 6 },
    BlockPos { x: 15, y: 70, z: 9 },
    BlockPos { x: 10, y: 70, z: 6 },
];

/// The 4 real Stone Buttons around one pedestal, one per side (N/E/S/W) - confirmed in the
/// capture, e.g. pedestal `(10,70,6)` has buttons at exactly these 4 offsets.
fn button_offsets(pedestal: BlockPos) -> [BlockPos; 4] {
    [
        BlockPos { x: pedestal.x, y: pedestal.y, z: pedestal.z - 1 }, // North
        BlockPos { x: pedestal.x + 1, y: pedestal.y, z: pedestal.z }, // East
        BlockPos { x: pedestal.x, y: pedestal.y, z: pedestal.z + 1 }, // South
        BlockPos { x: pedestal.x - 1, y: pedestal.y, z: pedestal.z }, // West
    ]
}

// --- Answer bank -----------------------------------------------------------------------------
// Ported from Odin's `quizAnswers.json` (re-fetched fresh, not trusted from memory). Wording
// variants for the same location/entity ("Spiders Den"/"Spider's Den", "Hub"/"The Hub") are
// deduped to one canonical question per fact. Each `(question, answer)` pair's wrong options are
// drawn from the *other* entries in the same array, so distractors always look plausible
// together (e.g. a Fairy Soul count question only ever shows other Fairy Soul counts as options).

const FAIRY_SOULS: &[(&str, &str)] = &[
    ("How many total Fairy Souls are there?", "289 Fairy Souls"),
    ("How many Fairy Souls are there in the Hub?", "80 Fairy Souls"),
    ("How many Fairy Souls are there in Backwater Bayou?", "5 Fairy Souls"),
    ("How many Fairy Souls are there in Spider's Den?", "19 Fairy Souls"),
    ("How many Fairy Souls are there in The End?", "12 Fairy Souls"),
    ("How many Fairy Souls are there in The Farming Islands?", "20 Fairy Souls"),
    ("How many Fairy Souls are there in Crimson Isle?", "29 Fairy Souls"),
    ("How many Fairy Souls are there in The Park?", "12 Fairy Souls"),
    ("How many Fairy Souls are there in Deep Caverns?", "21 Fairy Souls"),
    ("How many Fairy Souls are there in Jerry's Workshop?", "5 Fairy Souls"),
    ("How many Fairy Souls are there in Dungeon Hub?", "7 Fairy Souls"),
    ("How many Fairy Souls are there in Gold Mine?", "12 Fairy Souls"),
];

const STATUS: &[(&str, &str)] = &[
    ("What is the status of Scarf?", "Apprentice Necromancer"),
    ("What is the status of Bonzo?", "New Necromancer"),
    ("What is the status of Thorn?", "Shaman Necromancer"),
    ("What is the status of The Watcher?", "Stalker"),
    ("What is the status of Sadan?", "Necromancer Lord"),
    ("What is the status of Livid?", "Master Necromancer"),
    ("What is the status of The Professor?", "Professor"),
    ("What is the status of Maxor, Storm, Goldor, and Necron?", "The Wither Lords"),
];

// The source bank's own key for the Wool Weaver question is truncated ("...who sells stained")
// - completed here from real SkyBlock knowledge (the Hub's Wool Weaver NPC sells stained clay),
// not left broken since it's displayed as a real in-game chat line to players.
const MISC: &[(&str, &str)] = &[
    ("What is the name of the lady of the Nether?", "Elle"),
    ("Which brother is on the Spider's Den?", "Rick"),
    ("How many unique minions are there?", "61 Minions"),
    ("Which villager in the Village gives you a Rogue Sword?", "Jamie"),
    ("What is the name of Rick's brother?", "Pat"),
    ("What is the name of the vendor in the Hub who sells stained clay?", "Wool Weaver"),
    ("What is the name of the person that upgrades pets?", "Kat"),
];

/// "Which of these enemies does not spawn in the Spider's Den?" - Wither Skeleton is a
/// Catacombs-only mob, the other 5 are real Spider's Den spawns.
const SPIDER_DEN_CORRECT: &str = "Wither Skeleton";
const SPIDER_DEN_WRONG: &[&str] = &["Zombie Spider", "Cave Spider", "Dashing Spooder", "Broodfather", "Night Spider"];

/// "Which of these is not a dragon in The End?" - real SkyBlock Dragon fight has exactly 7 named
/// variants (confirmed via web search, cross-checked against the Hypixel Wiki: Protector, Wise,
/// Unstable, Superior, Strong, Young, Old) - "Elder Dragon" (this constant's previous value) does
/// NOT exist as a real variant at all, which meant every option this question ever showed was
/// fake with no actual correct answer to pick - the real bug behind "3 wrong answers, any pick is
/// cooked". `Holy`/`Older`/`Stable` were also dropped from the fake pool below - confusingly close
/// to real `Wise`/`Old`/`Unstable` without being an exact match either way.
const END_DRAGON_REAL: &[&str] = &[
    "Protector Dragon", "Wise Dragon", "Unstable Dragon", "Superior Dragon",
    "Strong Dragon", "Young Dragon", "Old Dragon",
];
const END_DRAGON_FAKE: &[&str] = &[
    "Zoomer Dragon", "Weak Dragon", "Stonk Dragon", "Boomer Dragon", "Booger Dragon", "Professor Dragon",
];

/// SkyBlock epoch (year 1 start) and real-seconds-per-SkyBlock-year, confirmed from Odin's
/// `QuizSolver.kt` - directly reusable, no lookup needed.
const SKYBLOCK_EPOCH_SECS: i64 = 1560276000;
const SKYBLOCK_YEAR_SECS: i64 = 446400;

fn current_skyblock_year() -> i64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    (now - SKYBLOCK_EPOCH_SECS) / SKYBLOCK_YEAR_SECS + 1
}

/// One logical question this puzzle can ask - an index into a shared-category pool, or one of
/// the hand-authored special cases. Sampled 3-at-a-time without replacement across every
/// category combined, so a single run never repeats a fact even across categories.
#[derive(Clone, Copy)]
enum QuestionKind {
    FairySouls(usize),
    Status(usize),
    Misc(usize),
    SpiderDenEnemy,
    EndDragon,
    NightSpawner,
    SkyblockYear,
}

fn all_question_kinds() -> Vec<QuestionKind> {
    let mut kinds = Vec::with_capacity(FAIRY_SOULS.len() + STATUS.len() + MISC.len() + 4);
    kinds.extend((0..FAIRY_SOULS.len()).map(QuestionKind::FairySouls));
    kinds.extend((0..STATUS.len()).map(QuestionKind::Status));
    kinds.extend((0..MISC.len()).map(QuestionKind::Misc));
    kinds.push(QuestionKind::SpiderDenEnemy);
    kinds.push(QuestionKind::EndDragon);
    kinds.push(QuestionKind::NightSpawner);
    kinds.push(QuestionKind::SkyblockYear);
    kinds
}

#[derive(Debug, Default, Clone)]
pub struct QuestionInstance {
    question: &'static str,
    /// Already shuffled - `correct_index` points at whichever slot ended up holding the answer.
    options: [String; 3],
    correct_index: usize,
}

/// Builds a shuffled `QuestionInstance` from a correct answer and 2 wrong ones, so every caller
/// doesn't have to re-implement the shuffle-and-track-the-index dance.
fn shuffled_instance(question: &'static str, correct: String, wrong: [String; 2], rng: &mut impl Rng) -> QuestionInstance {
    let mut options = [correct, wrong[0].clone(), wrong[1].clone()];
    let mut order = [0usize, 1, 2];
    order.shuffle(rng);
    let correct_index = order.iter().position(|&i| i == 0).unwrap();
    QuestionInstance {
        question,
        options: [
            std::mem::take(&mut options[order[0]]),
            std::mem::take(&mut options[order[1]]),
            std::mem::take(&mut options[order[2]]),
        ],
        correct_index,
    }
}

fn build_question(kind: QuestionKind, rng: &mut impl Rng) -> QuestionInstance {
    match kind {
        QuestionKind::FairySouls(i) => {
            let (question, answer) = FAIRY_SOULS[i];
            let wrong = other_two(FAIRY_SOULS, i, rng, |(_, a)| a.to_string());
            shuffled_instance(question, answer.to_string(), wrong, rng)
        }
        QuestionKind::Status(i) => {
            let (question, answer) = STATUS[i];
            let wrong = other_two(STATUS, i, rng, |(_, a)| a.to_string());
            shuffled_instance(question, answer.to_string(), wrong, rng)
        }
        QuestionKind::Misc(i) => {
            let (question, answer) = MISC[i];
            let wrong = other_two(MISC, i, rng, |(_, a)| a.to_string());
            shuffled_instance(question, answer.to_string(), wrong, rng)
        }
        QuestionKind::SpiderDenEnemy => {
            let mut wrong = SPIDER_DEN_WRONG.to_vec();
            wrong.shuffle(rng);
            shuffled_instance(
                "Which of these enemies does not spawn in the Spider's Den?",
                SPIDER_DEN_CORRECT.to_string(),
                [wrong[0].to_string(), wrong[1].to_string()],
                rng,
            )
        }
        QuestionKind::EndDragon => {
            // Exactly 1 fake (the correct "not a dragon" pick) + 2 genuinely real dragon
            // variants (both legitimately wrong picks) - the old version showed 2 fakes + 1
            // fabricated "real" name that wasn't actually real either, leaving no correct answer
            // at all. See `END_DRAGON_REAL`'s doc comment.
            let fake = END_DRAGON_FAKE.choose(rng).unwrap().to_string();
            let mut reals = END_DRAGON_REAL.to_vec();
            reals.shuffle(rng);
            shuffled_instance(
                "Which of these is not a dragon in The End?",
                fake,
                [reals[0].to_string(), reals[1].to_string()],
                rng,
            )
        }
        QuestionKind::NightSpawner => {
            // Zombie Villagers only spawn under night-time light conditions like a regular
            // Zombie; Ghasts have no day/night gating (the Nether has no day/night cycle), so
            // Ghast is the deliberate wrong option here rather than the correct one.
            let filler = MISC.choose(rng).unwrap().1.to_string();
            shuffled_instance(
                "Which of these monsters only spawns at night?",
                "Zombie Villager".to_string(),
                ["Ghast".to_string(), filler],
                rng,
            )
        }
        QuestionKind::SkyblockYear => {
            let year = current_skyblock_year();
            shuffled_instance(
                "What SkyBlock year is it?",
                format!("Year {year}"),
                [format!("Year {}", year - 1), format!("Year {}", year + 1)],
                rng,
            )
        }
    }
}

/// Picks 2 distinct entries from `pool` other than index `skip`, mapped through `pick`.
fn other_two<T>(pool: &[T], skip: usize, rng: &mut impl Rng, pick: impl Fn(&T) -> String) -> [String; 2] {
    let mut others: Vec<&T> = pool.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, v)| v).collect();
    others.shuffle(rng);
    [pick(others[0]), pick(others[1])]
}

// --- Puzzle state and flow ----------------------------------------------------------------------

/// Shared per-room puzzle state - jointly owned (`Rc<RefCell<_>>`) by the 12 buttons'
/// `BlockInteractAction::QuizButton` entries, same idiom `ThreeWeirdosState`/`TicTacToeState`
/// already use.
#[derive(Debug)]
pub struct QuizState {
    room_index: usize,
    questions: [QuestionInstance; 3],
    current: usize,
    /// World position of each answer's pedestal - the origin for that answer's reveal "pop"
    /// sound (see `reveal_answer_option`).
    pedestal_centers: [BlockPos; 3],
    /// The current question's 3 answer holograms, index-paired with `pedestal_centers` - `None`
    /// until `reveal_answer_option` actually places that one. Despawned as soon as the question
    /// is answered (see `despawn_holograms`), then repopulated fresh for the next question.
    hologram_entities: [Option<EntityId>; 3],
    /// True only once all 3 options for the current question have actually been revealed - false
    /// during the intro/transition delays and the staggered per-option reveal itself, and
    /// permanently false once the puzzle is resolved (solved or failed), so a stray or duplicate
    /// click no-ops instead of double-processing.
    awaiting_answer: bool,
}

/// Sets up the Quiz puzzle for `room` if it actually is one - picks 3 questions, resolves and
/// registers the 12 real buttons, and kicks off Oruo's intro speech. No-op for every other room
/// (belt-and-suspenders; the caller already filters by name and by `Room::quiz_started`). Called
/// once from `Dungeon::tick`'s room-entry hook, the first time a player actually crosses into
/// the room - same reasoning as `three_weirdos::setup`'s doc comment (Oruo shouldn't start
/// talking before anyone has actually entered).
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Quiz" {
        return;
    }

    let mut rng = seeded_rng();

    let kinds = all_question_kinds();
    let chosen: Vec<QuestionKind> = kinds.choose_multiple(&mut rng, 3).copied().collect();
    let mut questions = [QuestionInstance::default(), QuestionInstance::default(), QuestionInstance::default()];
    for (i, kind) in chosen.into_iter().enumerate() {
        questions[i] = build_question(kind, &mut rng);
    }

    let answer_buttons: [[BlockPos; 4]; 3] = std::array::from_fn(|i| {
        let local = button_offsets(PEDESTALS[i]);
        std::array::from_fn(|j| room.get_world_block_pos(&local[j]))
    });
    let pedestal_centers: [BlockPos; 3] = std::array::from_fn(|i| room.get_world_block_pos(&PEDESTALS[i]));

    let state = Rc::new(RefCell::new(QuizState {
        room_index,
        questions,
        current: 0,
        pedestal_centers,
        hologram_entities: [None; 3],
        awaiting_answer: false,
    }));

    for (answer_index, group) in answer_buttons.iter().enumerate() {
        for &pos in group {
            world.interactable_blocks.insert(pos, BlockInteractAction::QuizButton { state: state.clone(), answer_index });
        }
    }

    say(world, INTRO_LINES[0]);
    schedule_intro_line(world, state, 1);
}

/// Sends `world`-wide `[STATUE] Oruo the Omniscient: ` line - matches how every other puzzle
/// here broadcasts its own dungeon-wide PUZZLE SOLVED/FAILED lines (`three_weirdos.rs`), not
/// just to players physically standing in the room. The name/prefix is always dark red with a
/// white colon, and the message body defaults to white except for self-references and mentions
/// of The Catacombs, which `colorize_body` highlights inline (see its doc comment).
fn say(world: &mut World, line: &str) {
    let message = format!("§4[STATUE] Oruo the Omniscient§f: {}", colorize_body(line));
    for (_, player) in &mut world.players {
        player.send_message(&message);
    }
}

/// Highlights Oruo's own name (dark red) and any mention of The Catacombs (red) inside a spoken
/// line's body, reverting back to white immediately after each so the rest of the sentence stays
/// the default body color. Both substrings are plain English words, not user input, so a literal
/// substring replace is safe and doesn't need real tokenizing.
fn colorize_body(line: &str) -> String {
    let line = line.replace("Oruo the Omniscient", "§4Oruo the Omniscient§f");
    line.replace("The Catacombs", "§cThe Catacombs§f")
}

fn schedule_intro_line(world: &mut World, state: Rc<RefCell<QuizState>>, line: usize) {
    world.server_mut().schedule(STAGE_GAP_TICKS, move |server| {
        if line < INTRO_LINES.len() {
            say(&mut server.world, INTRO_LINES[line]);
            schedule_intro_line(&mut server.world, state, line + 1);
        } else {
            begin_question(&mut server.world, &state);
        }
    });
}

const LETTERS: [char; 3] = ['ⓐ', 'ⓑ', 'ⓒ'];
const LINE_WIDTH: usize = 75;
/// Real delay between the question prompt appearing and the first answer option being placed.
const FIRST_OPTION_DELAY_TICKS: u32 = 40;
/// Real gap between each subsequent option being placed - a player cannot answer until all 3
/// are placed (`QuizState::awaiting_answer` only flips true after the 3rd).
const OPTION_REVEAL_GAP_TICKS: u32 = 10;

/// Sends the full question chat block (blank/header/question/blank/3 options/blank, centered) -
/// the chat side of all 3 options arrives immediately with the question, only the physical
/// per-pedestal reveal (sound + hologram, see `reveal_answer_option`) is staggered. Schedules
/// the first option's staggered reveal.
fn begin_question(world: &mut World, state: &Rc<RefCell<QuizState>>) {
    let (question, options, number) = {
        let data = state.borrow();
        let q = &data.questions[data.current];
        (q.question, q.options.clone(), data.current + 1)
    };

    let header = format!("Question #{number}");
    for (_, player) in &mut world.players {
        player.send_message("");
        player.send_message(&center_colored(&header, LINE_WIDTH, "§6§l"));
        player.send_message(&center_colored(question, LINE_WIDTH, "§6"));
        player.send_message("");
        for (letter, option) in LETTERS.iter().zip(options.iter()) {
            player.send_message(&format!("    §6 {letter} §a{option}"));
        }
        player.send_message("");
    }

    let state = state.clone();
    world.server_mut().schedule(FIRST_OPTION_DELAY_TICKS, move |server| {
        reveal_answer_option(&mut server.world, &state, 0);
    });
}

/// Height above a pedestal's block position the answer hologram floats at - a reasonable
/// default, not captured/confirmed against a real client (see the module doc comment's "open
/// gaps" note inherited from the original research - the real hologram height was never
/// confirmed either way).
const HOLOGRAM_Y_OFFSET: f64 = 1.3;

/// Places answer option `index` (left to right: ⓐ, ⓑ, ⓒ) - plays the real "pop" placement sound
/// (`entity.item.pickup`'s legacy equivalent) and spawns a floating nametag hologram with the
/// answer text, both positioned at that answer's own pedestal. The chat lines already went out
/// with the question (see `begin_question`); this is just the physical reveal. Schedules the
/// next option 10 ticks later, or - once the 3rd option lands - flips `awaiting_answer` so
/// clicks start counting.
fn reveal_answer_option(world: &mut World, state: &Rc<RefCell<QuizState>>, index: usize) {
    let (option, pedestal) = {
        let mut data = state.borrow_mut();
        if index == 2 {
            data.awaiting_answer = true;
        }
        (data.questions[data.current].options[index].clone(), data.pedestal_centers[index])
    };

    let sound = SoundEffect {
        sound: Sounds::Pop.id(),
        pos_x: pedestal.x as f64 + 0.5,
        pos_y: pedestal.y as f64 + 0.5,
        pos_z: pedestal.z as f64 + 0.5,
        volume: 1.0,
        pitch: 0.587,
    };
    for (_, player) in &mut world.players {
        player.write_packet(&sound);
    }

    let hologram_pos = DVec3::new(pedestal.x as f64 + 0.5, pedestal.y as f64 + HOLOGRAM_Y_OFFSET, pedestal.z as f64 + 0.5);
    let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
    metadata.is_invisible = true;
    metadata.custom_name = Some(format!("§6{} §a{option}", LETTERS[index]));
    metadata.custom_name_visible = true;
    metadata.ai_disabled = true;
    metadata.is_small_armor_stand = true;
    if let Ok(entity_id) = world.spawn_entity(hologram_pos, metadata, NoEntityImpl) {
        state.borrow_mut().hologram_entities[index] = Some(entity_id);
    }

    if index < 2 {
        let state = state.clone();
        world.server_mut().schedule(OPTION_REVEAL_GAP_TICKS, move |server| {
            reveal_answer_option(&mut server.world, &state, index + 1);
        });
    }
}

/// Centers `text` within `width`, padding based on its own (uncolored) length, then prepends
/// `color` after the padding - color codes are invisible on the client so they must not count
/// toward the width used to compute how much padding to add.
fn center_colored(text: &str, width: usize, color: &str) -> String {
    let pad = width.saturating_sub(text.chars().count()) / 2;
    format!("{}{}{}", " ".repeat(pad), color, text)
}

/// Despawns whichever of the current question's 3 answer holograms actually got placed (a
/// question answered before the 3rd hologram lands is impossible - `awaiting_answer` blocks
/// clicks until then - but this stays correct either way) and clears the slots for the next
/// question. Called the instant a question is answered, correct or wrong.
fn despawn_holograms(world: &mut World, state: &Rc<RefCell<QuizState>>) {
    let entities = std::mem::replace(&mut state.borrow_mut().hologram_entities, [None; 3]);
    for entity_id in entities.into_iter().flatten() {
        world.despawn_entity(entity_id);
    }
}

/// `BlockInteractAction::QuizButton`'s handler - a player clicked the button at `block_pos`,
/// which belongs to answer slot `answer_index`.
pub fn interact_button(player: &mut Player, block_pos: &BlockPos, answer_index: usize, state: &Rc<RefCell<QuizState>>) {
    let (correct, current, room_index) = {
        let mut data = state.borrow_mut();
        if !data.awaiting_answer {
            return;
        }
        // Stop accepting further clicks immediately regardless of outcome - a wrong answer ends
        // the room, and a correct one is followed by a timed transition, not another click.
        data.awaiting_answer = false;
        let correct = answer_index == data.questions[data.current].correct_index;
        (correct, data.current, data.room_index)
    };

    despawn_holograms(player.world_mut(), state);

    let username = player.profile.username.clone();
    let pos = *block_pos;

    let click_sound = SoundEffect {
        sound: Sounds::RandomClick.id(),
        pos_x: pos.x as f64 + 0.5,
        pos_y: pos.y as f64 + 0.5,
        pos_z: pos.z as f64 + 0.5,
        volume: 0.3,
        pitch: 0.6,
    };
    for other in player.server_mut().world.players.values_mut() {
        other.write_packet(&click_sound);
    }

    if correct {
        let is_final = current == 2;
        // Must keep the real "[STATUE] Oruo the Omniscient: " prefix and the literal substrings
        // "answered the final question" / "answered Question #" - Odin's real QuizSolver only
        // clears its old answer highlight (and only detects puzzle completion) when a message
        // starts with that exact prefix and ends with "correctly!". Dropping the prefix for the
        // green/gold styling silently broke both, which is what caused the marker to never
        // disappear on the next question. `say()` supplies the prefix; the message text itself
        // still gets the green/gold styling on top.
        let message = if is_final {
            format!("§a{username} answered the final question correctly!")
        } else {
            format!("§a{username} answered §6Question #{} §acorrectly!", current + 1)
        };
        say(player.world_mut(), &message);

        let chime_pos = pos;
        player.server_mut().schedule(CORRECT_CHIME_DELAY_TICKS, move |server| {
            let chime = SoundEffect {
                sound: Sounds::LevelUp.id(),
                pos_x: chime_pos.x as f64 + 0.5,
                pos_y: chime_pos.y as f64 + 0.5,
                pos_z: chime_pos.z as f64 + 0.5,
                volume: 1.0,
                pitch: 0.746,
            };
            for other in server.world.players.values_mut() {
                other.write_packet(&chime);
            }
        });

        if is_final {
            let state = state.clone();
            player.server_mut().schedule(COMPLETION_GAP_TICKS, move |server| {
                complete_quiz(server, room_index, &state);
            });
        } else {
            let state = state.clone();
            player.server_mut().schedule(STAGE_GAP_TICKS, move |server| {
                let line = if current == 0 {
                    "2 questions left... Then you will have proven your worth to me!"
                } else {
                    "One more question!"
                };
                say(&mut server.world, line);

                let state = state.clone();
                server.schedule(STAGE_GAP_TICKS, move |server| {
                    state.borrow_mut().current += 1;
                    begin_question(&mut server.world, &state);
                });
            });
        }
    } else {
        say(player.world_mut(), &format!(
            "§a{username}§f chose the wrong answer! I shall never forget this moment of misrememberance."
        ));

        let fail_message = format!("§c§lPUZZLE FAIL! §a{username} §efailed the Quiz puzzle!");
        for other in player.server_mut().world.players.values_mut() {
            other.send_message(&fail_message);
        }

        player.server_mut().dungeon.record_puzzle_failed();
        if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
            room.puzzle_completed = true;
            room.puzzle_failed = true;
        }
        player.server_mut().dungeon.update_map_for_room(room_index);
    }
}

/// Runs once the final correct answer's completion delay elapses - sends the real flavor/reward
/// lines and marks the room solved. Flavor-only: this codebase has no mechanical stat/buff
/// system to hook a real "Blessing of Time" effect into, so (like every other puzzle's reward
/// here) this reproduces the real chat, not a simulated stat change. Unlike every other puzzle,
/// real Quiz never sends a "PUZZLE SOLVED!" banner - the reward lines below are the only
/// confirmation, so none is added here.
fn complete_quiz(server: &mut Server, room_index: usize, state: &Rc<RefCell<QuizState>>) {
    let _ = state; // kept for symmetry/future use; nothing further needed from it here.

    say(&mut server.world, "I bestow upon you all the power of a hundred years!");
    for (_, player) in &mut server.world.players {
        player.send_message("§bDUNGEON BUFF! §fYou found a §b§lBlessing of Time V§f!");
        player.send_message("     Granted you +34.3 & +1.17x HP, +34.3 & +1.17x Defense, +34.3 & +1.17x Intelligence, and +34.3 & +1.17x Strength.");
    }

    if let Some(room) = server.dungeon.rooms.get_mut(room_index) {
        room.puzzle_completed = true;
    }
    server.dungeon.update_map_for_room(room_index);
}

#[cfg(test)]
mod verify_positions {
    use super::*;
    use crate::dungeon::room::room_data::RoomData;
    use crate::server::block::blocks::Blocks;
    use std::collections::HashSet;

    /// Regression test: confirms `PEDESTALS`/`button_offsets` still match this project's own
    /// captured Quiz room data, not a value borrowed/assumed from elsewhere - see the mixup this
    /// guarded against (coordinates were briefly cross-checked against a different project's
    /// capture pipeline before this test existed).
    #[test]
    fn button_positions_match_real_room_capture() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/room_data/rooms/18,quiz,-60,-600.json");
        let raw = std::fs::read_to_string(path).unwrap();
        let data = RoomData::from_raw_json(&raw);

        let mut real_buttons: HashSet<BlockPos> = HashSet::new();
        for (index, block) in data.block_data.iter().enumerate() {
            if matches!(block, Blocks::StoneButton { .. }) {
                let index = index as i32;
                let x = index % data.width;
                let z = (index / data.width) % data.length;
                let y = data.bottom + index / (data.width * data.length);
                real_buttons.insert(BlockPos { x, y, z });
            }
        }

        let expected: HashSet<BlockPos> = PEDESTALS.iter().flat_map(|&p| button_offsets(p)).collect();
        assert_eq!(expected, real_buttons, "quiz.rs's hardcoded button positions no longer match the captured Quiz room");
    }
}
