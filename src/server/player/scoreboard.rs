use crate::net::packets::client_bound::display_scoreboard::{DisplayScoreboard, SIDEBAR};
use crate::net::packets::client_bound::scoreboard_objective::{ScoreboardObjective, ScoreboardRenderType, ADD_OBJECTIVE, UPDATE_NAME};
use crate::net::packets::client_bound::teams::{Teams, ADD_PLAYER, CREATE_TEAM, REMOVE_TEAM, UPDATE_TEAM};
use crate::net::packets::client_bound::update_score::{UpdateScore, UpdateScoreAction};
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::utils::sized_string::SizedString;

const OBJECTIVE_NAME: &str = "SBScoreboard";

/// wrapper for Vec<SizedString<32>>,
/// which includes functions to push strings (and strs), splitting their lines and converting to sized strings
#[derive(Debug)]
pub struct ScoreboardLines(pub Vec<SizedString<32>>);

impl ScoreboardLines {
    #[inline(always)]
    pub fn push(&mut self, string: String) {
        self.0.extend(string.lines().map(|str| str.into()));
    }
    #[inline(always)]
    pub fn push_str(&mut self, string: &str) {
        self.0.extend(string.lines().map(|str| str.into()));
    }
    #[inline(always)]
    pub fn new_line(&mut self) {
        self.0.push("".into());
    }
}

/// Scoreboard
///
/// Contains a list of strings, which are sent to the client to display on their scoreboard.
/// This is designed so that the scoreboard can be updated in one go per tick, using [Scoreboard::update].
#[derive(Debug)]
pub struct Scoreboard {
    lines: ScoreboardLines,
}

type Packet = ClientBoundPacket;

impl Scoreboard {

    pub fn new() -> Scoreboard {
        Scoreboard {
            lines: ScoreboardLines(Vec::new()),
        }
    }

    /// must be sent to client when it is initialized
    /// if not sent scoreboard will not display on the client.
    pub fn packets_to_init(&self) -> Vec<Packet> {
        vec![
            Packet::from(ScoreboardObjective {
                objective_name: OBJECTIVE_NAME.into(),
                objective_value: OBJECTIVE_NAME.into(),
                typ: ScoreboardRenderType::Integer,
                mode: ADD_OBJECTIVE
            }),
            Packet::from(DisplayScoreboard {
                position: SIDEBAR,
                score_name: OBJECTIVE_NAME.into(),
            })
        ]

    }

    /// this function updates the scoreboard with a new list
    /// and sends packets according to what has changed
    ///
    /// the first index always represents the header
    pub fn update(&mut self, lines: ScoreboardLines) -> Vec<Packet> {

        // this isn't the most optimal/performant solution, but it works and its convent
        let mut packets = Vec::new();
        
        let (old_len, new_len) = (self.lines.0.len(), lines.0.len());
        let is_size_different = new_len != old_len;
        
        if is_size_different && old_len != 0 {
            for (i, _) in self.lines.0.iter().enumerate() {
                // skip header
                if i == 0 {
                    continue;
                }
                
                let name = format!("{}", i);
                
                packets.push(Packet::from(UpdateScore {
                    name: hide_name(&name),
                    objective: OBJECTIVE_NAME.into(),
                    value: 0,
                    action: UpdateScoreAction::Remove,
                }));
                packets.push(Packet::from(Teams {
                    name: "".into(),
                    display_name: "".into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![],
                    action: REMOVE_TEAM,
                    friendly_flags: 0,
                }))
            }
        }

        for i in 0..new_len {
            let new_str = &lines.0[i];
            // might be out of index from size changing, cant use [] to access
            let old_str = &self.lines.0.get(i);

            if old_str.is_some_and(|str| str == new_str) && !is_size_different {
                continue;
            }
            // index 0 is always header
            if i == 0 {
                packets.push(Packet::from(ScoreboardObjective {
                    objective_name: OBJECTIVE_NAME.into(),
                    objective_value: new_str.clone().into(),
                    typ: ScoreboardRenderType::Integer,
                    mode: UPDATE_NAME,
                }))
            } else {
                let line_index = new_len - i;
                let name = format!("{}", i);
                let team = format!("team_{}", line_index);

                let first_half = first_half(new_str);
                
                if is_size_different {
                    packets.push(Packet::from(Teams {
                        name: team.clone().into(),
                        display_name: team.clone().into(),
                        prefix: "".into(),
                        suffix: "".into(),
                        name_tag_visibility: "always".into(),
                        color: 15,
                        players: vec![],
                        action: CREATE_TEAM,
                        friendly_flags: 3,
                    }));

                    packets.push(ClientBoundPacket::from(UpdateScore {
                        name: hide_name(&name),
                        objective: OBJECTIVE_NAME.into(),
                        value: line_index as i32,
                        action: UpdateScoreAction::Change,
                    }));
                }
                
                packets.push(Packet::from(Teams {
                    name: team.clone().into(),
                    display_name: team.clone().into(),
                    prefix: first_half,
                    suffix: second_half(new_str),
                    name_tag_visibility: "always".into(),
                    color: 15,
                    players: vec![],
                    action: UPDATE_TEAM,
                    friendly_flags: 3,
                }));
                
                if is_size_different {
                    packets.push(ClientBoundPacket::from(Teams {
                        name: team.clone().into(),
                        display_name: team.into(),
                        prefix: "".into(),
                        suffix: "".into(),
                        name_tag_visibility: "always".into(),
                        color: -1,
                        players: vec![hide_name(&name)],
                        action: ADD_PLAYER,
                        friendly_flags: 0,
                    }));
                }
            }
        }
        self.lines = lines;
        packets
    }

}

fn hide_name(key: &str) -> SizedString<40> {
    let mut result = String::new();
    for char in key.chars() {
        result.push('§');
        result.push(char);
    }
    result.push_str("§r");
    result.into()
}

fn first_half(string: &SizedString<32>) -> SizedString<16> {
    let string = string.as_str();
    string.chars().take(16).collect::<String>().into()
}

pub fn second_half(string: &SizedString<32>) -> SizedString<16> {
    // this kind of hurts me but im too lazy to try to optimize this
    let mut result = string.as_str().chars().skip(16).collect::<String>();
    if let Some(c) = get_last_color_code(&first_half(string)) {
        result = format!("§{}{}", c, result)
    }
    result.into()
}

fn get_last_color_code(string: &SizedString<16>) -> Option<char> {
    let string = string.as_str();
    let mut chars = string.chars().rev().peekable();
    while let Some(c) = chars.next() {
        if c != '§' {
            if let Some('§') = chars.peek() {
                return Some(c)
            }
        }
    }
    None
}