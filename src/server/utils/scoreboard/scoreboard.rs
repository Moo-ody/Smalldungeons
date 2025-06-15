use crate::net::packets::client_bound::display_scoreboard::{DisplayScoreboard, SIDEBAR};
use crate::net::packets::client_bound::scoreboard_objective::{ScoreboardObjective, ScoreboardRenderType, ADD_OBJECTIVE, UPDATE_NAME};
use crate::net::packets::client_bound::teams::{Teams, ADD_PLAYER, CREATE_TEAM, REMOVE_TEAM, UPDATE_TEAM};
use crate::net::packets::client_bound::update_score::{UpdateScore, UpdateScoreAction};
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::utils::scoreboard::SizedString;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::convert::Into;

pub const OBJECTIVE_NAME: &str = "SBScoreboard";

#[derive(Debug)]
pub struct Scoreboard {
    header: SizedString<32>,
    lines: IndexMap<String, ScoreboardLine>,
    to_add: Vec<(Option<usize>, (String, ScoreboardLine))>,
    to_remove: HashSet<String>,

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
            to_add: Vec::new(),
            to_remove: HashSet::new(),
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
        self.to_remove.insert(key.into());
    }

    pub fn remove_line_at(&mut self, line: usize) {
        if let Some(entry) = self.lines.get_index_entry(line) {
            self.to_remove.insert(entry.key().clone());
        }
    }

    pub fn add_line(&mut self, key: impl Into<String>, line: impl Into<SizedString<32>>) {
        self.to_add.push((None, (key.into(), ScoreboardLine::new(line))));
    }

    pub fn add_line_at(&mut self, index: usize, key: impl Into<String>, line: impl Into<SizedString<32>>) {
        self.to_add.push((Some(index), (key.into(), ScoreboardLine::new(line))));
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

        let dirty = !self.to_remove.is_empty() || !self.to_add.is_empty();

        if dirty {
            let len = self.lines.len();
            for (index, key) in self.lines.keys().enumerate() {
                packets.push(ClientBoundPacket::from(UpdateScore {
                    name: hide_key(key),
                    objective: OBJECTIVE_NAME.into(),
                    value: 0,
                    action: UpdateScoreAction::Remove,
                }));

                packets.push(ClientBoundPacket::from(Teams {
                    name: format!("team_{}", len - index).into(),
                    display_name: "".into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![],
                    action: REMOVE_TEAM,
                    friendly_flags: 0,
                }));
            }
        }

        self.lines.retain(|key, _| !self.to_remove.contains(key));
        self.to_remove.clear();

        for (index, (key, line)) in self.to_add.drain(..) {
            if let Some(index) = index {
                self.lines.shift_insert(index, key, line);
            } else {
                self.lines.insert(key, line);
            }
        }

        let len = self.lines.len();
        for (index, (key, line)) in self.lines.iter_mut().enumerate() {
            if !line.dirty && !dirty {
                continue;
            }
            let line_index = len - index - 1;
            let team = format!("team_{}", line_index + 1);

            if dirty {
                packets.push(ClientBoundPacket::from(Teams {
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
                    name: hide_key(key),
                    objective: OBJECTIVE_NAME.into(),
                    value: line_index as i32,
                    action: UpdateScoreAction::Change,
                }));
            }

            packets.push(ClientBoundPacket::from(Teams {
                name: team.clone().into(),
                display_name: team.clone().into(),
                prefix: line.first_half(),
                suffix: line.second_half(),
                name_tag_visibility: "always".into(),
                color: 15,
                players: vec![],
                action: UPDATE_TEAM,
                friendly_flags: 3,
            }));

            if dirty {
                packets.push(ClientBoundPacket::from(Teams {
                    name: team.clone().into(),
                    display_name: team.into(),
                    prefix: "".into(),
                    suffix: "".into(),
                    name_tag_visibility: "always".into(),
                    color: -1,
                    players: vec![hide_key(key)],
                    action: ADD_PLAYER,
                    friendly_flags: 0,
                }));
            }

            line.dirty = false;
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