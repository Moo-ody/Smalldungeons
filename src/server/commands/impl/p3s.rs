use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::world::World;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::net::protocol::play::clientbound::Chat;

pub struct P3S;

fn spawn_all_p3_armor_stands(world: &mut World) {
    // Spawn armor stands for all sections and devices
    let all_terminals = vec![
        // S1 Terminals
        crate::dungeon::p3::terminal::Terminal::new(1, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(110, 112, 73)),
        crate::dungeon::p3::terminal::Terminal::new(2, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(110, 118, 79)),
        crate::dungeon::p3::terminal::Terminal::new(3, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(90, 111, 92)),
        crate::dungeon::p3::terminal::Terminal::new(4, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(90, 121, 101)),
        crate::dungeon::p3::terminal::Terminal::new(5, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(94, 122, 113)).with_target(crate::server::block::block_position::BlockPos::new(94, 124, 113)),
        crate::dungeon::p3::terminal::Terminal::new(6, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(106, 122, 113)).with_target(crate::server::block::block_position::BlockPos::new(106, 124, 113)),
        
        // S2 Terminals
        crate::dungeon::p3::terminal::Terminal::new(7, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(68, 108, 122)),
        crate::dungeon::p3::terminal::Terminal::new(8, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(59, 119, 123)),
        crate::dungeon::p3::terminal::Terminal::new(9, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(47, 108, 122)),
        crate::dungeon::p3::terminal::Terminal::new(10, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(39, 107, 142)),
        crate::dungeon::p3::terminal::Terminal::new(11, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(40, 123, 123)),
        crate::dungeon::p3::terminal::Terminal::new(12, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(27, 122, 127)).with_target(crate::server::block::block_position::BlockPos::new(27, 124, 127)),
        crate::dungeon::p3::terminal::Terminal::new(13, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(23, 130, 138)).with_target(crate::server::block::block_position::BlockPos::new(23, 132, 138)),
        
        // S3 Terminals
        crate::dungeon::p3::terminal::Terminal::new(14, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(-2, 108, 112)),
        crate::dungeon::p3::terminal::Terminal::new(15, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(-2, 118, 93)),
        crate::dungeon::p3::terminal::Terminal::new(16, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(18, 122, 93)),
        crate::dungeon::p3::terminal::Terminal::new(17, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(-2, 108, 77)),
        crate::dungeon::p3::terminal::Terminal::new(18, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(14, 120, 55)).with_target(crate::server::block::block_position::BlockPos::new(14, 122, 55)),
        crate::dungeon::p3::terminal::Terminal::new(19, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(2, 120, 55)).with_target(crate::server::block::block_position::BlockPos::new(2, 122, 55)),
        
        // S4 Terminals
        crate::dungeon::p3::terminal::Terminal::new(20, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(41, 108, 30)),
        crate::dungeon::p3::terminal::Terminal::new(21, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(44, 120, 30)),
        crate::dungeon::p3::terminal::Terminal::new(22, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(67, 108, 30)),
        crate::dungeon::p3::terminal::Terminal::new(23, crate::dungeon::p3::terminal::TerminalType::Terminal, crate::server::block::block_position::BlockPos::new(72, 114, 47)),
        crate::dungeon::p3::terminal::Terminal::new(24, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(84, 119, 34)).with_target(crate::server::block::block_position::BlockPos::new(84, 121, 34)),
        crate::dungeon::p3::terminal::Terminal::new(25, crate::dungeon::p3::terminal::TerminalType::Lever, crate::server::block::block_position::BlockPos::new(86, 126, 46)).with_target(crate::server::block::block_position::BlockPos::new(86, 128, 46)),
        
        // Devices
        crate::dungeon::p3::terminal::Terminal::new(100, crate::dungeon::p3::terminal::TerminalType::SimonSays, crate::server::block::block_position::BlockPos::new(110, 118, 91)),
        crate::dungeon::p3::terminal::Terminal::new(101, crate::dungeon::p3::terminal::TerminalType::Lamps, crate::server::block::block_position::BlockPos::new(60, 130, 142)),
        crate::dungeon::p3::terminal::Terminal::new(102, crate::dungeon::p3::terminal::TerminalType::Align, crate::server::block::block_position::BlockPos::new(-2, 118, 74)),
        crate::dungeon::p3::terminal::Terminal::new(103, crate::dungeon::p3::terminal::TerminalType::ShootTarget, crate::server::block::block_position::BlockPos::new(63, 125, 34)),
    ];
    
    for mut terminal in all_terminals {
        terminal.spawn_armor_stands(world);
    }
}

impl CommandMetadata for P3S {
    const NAME: &'static str = "p3s";

    fn run(world: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> {
        // Spawn all P3 armor stands (terminals and devices)
        spawn_all_p3_armor_stands(world);
        
        // Send the boss message with proper color formatting
        let boss_message = ChatComponentTextBuilder::new("[BOSS] Goldor: ")
            .color(MCColors::DarkRed)
            .bold()
            .append(
                ChatComponentTextBuilder::new("Who dares trespass into my domain?")
                    .color(MCColors::Red)
                    .build()
            )
            .build();
        
        player.write_packet(&Chat {
            component: boss_message,
            chat_type: 0
        });
        
        // Schedule the Simon Says puzzle start for 20 ticks later
        let start_tick = world.tick_count + 20;
        world.simon_says.pending_actions.push((
            start_tick,
            crate::dungeon::p3::simon_says::SolutionAction::StartPuzzle
        ));
        
        // Send immediate confirmation message
        let confirm_message = ChatComponentTextBuilder::new("Simon Says puzzle will start in 1 second...")
            .color(MCColors::Yellow)
            .build();
        
        player.write_packet(&Chat {
            component: confirm_message,
            chat_type: 0
        });
        
        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        Vec::new() // No arguments needed
    }
}
