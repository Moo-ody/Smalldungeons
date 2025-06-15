use std::collections::{HashMap, HashSet};
use std::convert::Into;
use std::ops::Index;
use indexmap::{IndexMap, IndexSet};
use indexmap::map::MutableKeys;
use indexmap::set::MutableValues;
use crate::net::packets::client_bound::display_scoreboard::{DisplayScoreboard, SIDEBAR};
use crate::net::packets::client_bound::scoreboard_objective::{ScoreboardObjective, ScoreboardRenderType, ADD_OBJECTIVE, UPDATE_NAME};
use crate::net::packets::client_bound::teams::{Teams, ADD_PLAYER, CREATE_TEAM, REMOVE_TEAM, UPDATE_TEAM};
use crate::net::packets::client_bound::update_score::{UpdateScore, UpdateScoreAction};
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::utils::scoreboard::SizedString;

pub const OBJECTIVE_NAME: &str = "SBScoreboard";

#[derive(Debug)]
pub struct Scoreboard {
    header: SizedString<32>,
    lines: IndexMap<String, ScoreboardLine>,
    prev_lines: IndexMap<String, ScoreboardLine>,

    pub line_dirty: bool,
    pub header_dirty: bool,
    pub displaying: bool,
}

#[derive(Debug, Clone)]
pub struct ScoreboardLine {
    pub line: SizedString<32>,
    pub dirty: bool,
    pub new: bool,
}

impl ScoreboardLine {
    pub fn new(line: impl Into<SizedString<32>>) -> Self {
        Self {
            line: line.into(),
            dirty: true,
            new: true,
        }
    }

    pub fn first_half(&self) -> SizedString<16> {
        let string = self.line.as_str();
        string.chars().take(16).collect::<String>().into()
    }

    pub fn second_half(&self) -> SizedString<16> {
        let string = self.line.as_str();
        string.chars().skip(16).collect::<String>().into()
    }
}

impl Scoreboard {
    pub fn new(header: impl Into<SizedString<32>>) -> Self {
        Self {
            header: header.into(),
            lines: IndexMap::new(),
            prev_lines: IndexMap::new(),
            line_dirty: true,
            header_dirty: true,
            displaying: false,
        }
    }

    /// this needs to be sent after the first header packet, but before any header update packets.
    pub fn display_packet(&mut self) -> DisplayScoreboard {
        self.displaying = true;
        DisplayScoreboard {
            position: SIDEBAR,
            score_name: OBJECTIVE_NAME.into(),
        }
    }

    pub fn update_header(&mut self, header: impl Into<SizedString<32>>) {
        self.header = header.into();
        self.header_dirty = true;
    }

    pub fn update_line(&mut self, key: impl Into<String>, new_line: impl Into<SizedString<32>>) {
        if let Some(line) = self.lines.get_mut(&key.into()) {
            line.line = new_line.into();
            line.dirty = true;
        }
    }

    pub fn remove_line(&mut self, key: impl Into<String>) {
        self.lines.shift_remove(&key.into());
        self.line_dirty = true;
    }

    pub fn remove_line_at(&mut self, line: usize) {
        self.lines.shift_remove_index(line);
        self.line_dirty = true;
    }

    pub fn add_line(&mut self, key: impl Into<String>, line: impl Into<SizedString<32>>) {
        self.lines.insert(key.into(), ScoreboardLine::new(line));
        self.line_dirty = true;
    }

    pub fn add_line_at(&mut self, index: usize, key: impl Into<String>, line: impl Into<SizedString<32>>) {
        self.lines.shift_insert(index, key.into(), ScoreboardLine::new(line));
        self.line_dirty = true;
    }

    pub fn header_packet(&mut self) -> ScoreboardObjective {
        self.header_dirty = false;

        let mode = if self.displaying {
            UPDATE_NAME
        } else {
            ADD_OBJECTIVE
        };

        ScoreboardObjective {
            objective_name: OBJECTIVE_NAME.into(),
            objective_value: self.header.clone(),
            typ: ScoreboardRenderType::Integer,
            mode,
        }
    }

    pub fn get_packets(&mut self) -> Vec<ClientBoundPacket> {
        let mut packets = Vec::new();

        let mut line_index = self.prev_lines.len() as i32;
        if self.line_dirty {
            self.prev_lines.retain(|key, line| {
                if self.lines.contains_key(key) {
                    line_index -= 1;
                    return true;
                }

                let packet = Teams {
                    name: format!("team_{line_index}").into(),
                    display_name: "".into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![],
                    action: REMOVE_TEAM,
                    friendly_flags: 0,
                };


                packets.push(ClientBoundPacket::from(packet));

                let packet = UpdateScore {
                    name: hide_key(key),
                    objective: "SBScoreboard".into(),
                    value: 0,
                    action: UpdateScoreAction::Remove,
                };
                packets.push(ClientBoundPacket::from(packet));

                line_index -= 1;
                false
            })
        }

        let mut line_index = self.lines.len() as i32;
        for (key, line) in self.lines.iter_mut() {
            if !line.dirty && !self.line_dirty {
                continue;
            }

            if line.new || self.line_dirty {
                let packet = Teams {
                    name: format!("team_{line_index}").into(),
                    display_name: format!("team_{line_index}").into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: 15,
                    players: vec![],
                    action: CREATE_TEAM,
                    friendly_flags: 3,
                };

                packets.push(ClientBoundPacket::from(packet));

                let packet = UpdateScore {
                    name: hide_key(key),
                    objective: OBJECTIVE_NAME.into(),
                    value: line_index - 1,
                    action: UpdateScoreAction::Change,
                };
                packets.push(ClientBoundPacket::from(packet));
            }

            let packet = Teams {
                name: format!("team_{line_index}").into(),
                display_name: format!("team_{line_index}").into(),
                prefix: line.first_half(),
                suffix: line.second_half(),
                name_tag_visibility: "always".into(),
                color: 15,
                players: vec![],
                action: UPDATE_TEAM,
                friendly_flags: 3,
            };

            packets.push(ClientBoundPacket::from(packet));

            if line.new || self.line_dirty {
                let packet = Teams {
                    name: format!("team_{line_index}").into(),
                    display_name: format!("team_{line_index}").into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![hide_key(key)],
                    action: ADD_PLAYER,
                    friendly_flags: 0,
                };
                packets.push(ClientBoundPacket::from(packet));
            }

            line_index -= 1;

            line.dirty = false;
            line.new = false;
            self.prev_lines.insert(key.clone(), line.clone());
        }

        packets
    }
}

fn hide_key(key: &str) -> SizedString<40> {
    let mut result = String::new();
    for c in key.chars() {
        result.push('§');
        result.push(c);
    }
    result.push_str("§r");
    result.into()
}