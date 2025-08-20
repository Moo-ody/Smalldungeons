use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{DisplayScoreboard, ScoreboardObjective, Teams, UpdateScore};
use crate::net::var_int::VarInt;
use crate::server::utils::sized_string::SizedString;

const OBJECTIVE_NAME: &str = "SBScoreboard";

// for team packet:
pub const CREATE_TEAM: i8 = 0;
pub const REMOVE_TEAM: i8 = 1;
pub const UPDATE_TEAM: i8 = 2;
pub const ADD_PLAYER: i8 = 3;

pub const REMOVE_PLAYER: i8 = 4;

// for scoreboard objective packet
pub const ADD_OBJECTIVE: i8 = 0;
pub const UPDATE_NAME: i8 = 2;

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

impl Scoreboard {

    pub fn new() -> Scoreboard {
        Scoreboard {
            lines: ScoreboardLines(Vec::new()),
        }
    }

    /// must be sent to client when it is initialized
    /// if not sent scoreboard will not display on the client.
    pub fn write_init_packets(&self, packet_buffer: &mut PacketBuffer) {
        packet_buffer.write_packet(&ScoreboardObjective {
            objective_name: OBJECTIVE_NAME.into(),
            objective_value: OBJECTIVE_NAME.into(),
            render_type: "integer",
            mode: 0,
        });
        packet_buffer.write_packet(&DisplayScoreboard {
            position: 1,
            score_name: OBJECTIVE_NAME.into(),
        });
    }

    /// this function updates the scoreboard with a new list
    /// and sends packets according to what has changed
    ///
    /// the first index always represents the header
    pub fn write_update(&mut self, lines: ScoreboardLines, packet_buffer: &mut PacketBuffer) {
        
        let (old_len, new_len) = (self.lines.0.len(), lines.0.len());
        let is_size_different = new_len != old_len;
        
        if is_size_different && old_len != 0 {
            for (i, _) in self.lines.0.iter().enumerate() {
                // skip header
                if i == 0 {
                    continue;
                }
                
                let name = format!("{}", i);
                let team = format!("team_{}", old_len - i);
                
                packet_buffer.write_packet(&UpdateScore {
                    name: hide_name(&name),
                    objective: OBJECTIVE_NAME.into(),
                    value: VarInt(0),
                    action: VarInt(1),
                });
                packet_buffer.write_packet(&Teams {
                    name: team.into(),
                    display_name: "".into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![],
                    action: REMOVE_TEAM,
                    friendly_flags: 0,
                });
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
                packet_buffer.write_packet(&ScoreboardObjective {
                    objective_name: OBJECTIVE_NAME.into(),
                    objective_value: new_str.clone(),
                    render_type: "integer",
                    mode: UPDATE_NAME,
                });
            } else {
                let line_index = new_len - i;
                let name = format!("{}", i);
                let team = format!("team_{}", line_index);

                let (first_half, second_half) = split_string(new_str);
                
                if is_size_different {
                    packet_buffer.write_packet(&Teams {
                        name: team.clone().into(),
                        display_name: team.clone().into(),
                        prefix: "".into(),
                        suffix: "".into(),
                        name_tag_visibility: "always".into(),
                        color: 15,
                        players: vec![],
                        action: CREATE_TEAM,
                        friendly_flags: 3,
                    });
                    packet_buffer.write_packet(&UpdateScore {
                        name: hide_name(&name),
                        objective: OBJECTIVE_NAME.into(),
                        value: VarInt(line_index as i32),
                        action: VarInt(0),
                    });
                }
                
                packet_buffer.write_packet(&Teams {
                    name: team.clone().into(),
                    display_name: team.clone().into(),
                    prefix: first_half,
                    suffix: second_half,
                    name_tag_visibility: "always".into(),
                    color: 15,
                    players: vec![],
                    action: UPDATE_TEAM,
                    friendly_flags: 3,
                });
                
                if is_size_different {
                    packet_buffer.write_packet(&Teams {
                        name: team.clone().into(),
                        display_name: team.into(),
                        prefix: "".into(),
                        suffix: "".into(),
                        name_tag_visibility: "always".into(),
                        color: -1,
                        players: vec![hide_name(&name)],
                        action: ADD_PLAYER,
                        friendly_flags: 0,
                    });
                }
            }
        }
        self.lines = lines;
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

fn split_string(string: &SizedString<32>) -> (SizedString<16>, SizedString<16>) {
    let mut first_half = String::with_capacity(16);
    let mut second_half = String::with_capacity(16);
    let mut last_char = None;
    let mut last_color_code = None;
    for (i, c) in string.chars().enumerate() {
        if i < 16 {
            if last_char == Some('§') {
                last_color_code = Some(c);
            }
            last_char = Some(c);
            first_half.push(c);
        } else {
            if let Some(last_code) = last_color_code {
                second_half.push('§');
                second_half.push(last_code);
                last_color_code = None;
            }
            second_half.push(c);
        }
    }

    (first_half.into(), second_half.into())
}

// fn first_half(string: &SizedString<32>) -> SizedString<16> {
//     string.chars().take(16).collect::<String>().into()
// }
// 
// pub fn second_half(string: &SizedString<32>) -> SizedString<16> {
//     // this kind of hurts me but im too lazy to try to optimize this
//     let mut result = string.chars().skip(16).collect::<String>();
//     if let Some(c) = get_last_color_code(&first_half(string)) {
//         result = format!("§{}{}", c, result)
//     }
//     result.into()
// }

// fn get_last_color_code(string: &SizedString<16>) -> Option<char> {
//     let mut chars = string.chars().rev().peekable();
//     while let Some(c) = chars.next() {
//         if c != '§' && chars.peek() == Some(&'§') {
//             return Some(c)
//         }
//     }
//     None
// }