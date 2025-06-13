use std::os::linux::raw;

use serde_json::{json, Value};

use crate::server::block::blocks::Blocks;

#[derive(Debug)]
pub enum RoomShape {
    OneByOne,
    OneByTwo,
    OneByThree,
    OneByFour,
    TwoByTwo,
    L,
}

impl RoomShape {
    pub fn from_str(value: &str) -> RoomShape {
        match value {
            "1x1" => Self::OneByOne,
            "1x2" => Self::OneByTwo,
            "1x3" => Self::OneByThree,
            "1x4" => Self::OneByFour,
            "2x2" => Self::TwoByTwo,
            "L" => Self::L,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum RoomType {
    Normal,
    Puzzle,
    Trap,
    Fairy,
    Entrance,
    Blood,
    Yellow,
}

impl RoomType {
    pub fn from_str(value: &str) -> RoomType {
        match value {
            "normal" => RoomType::Normal,
            "puzzle" => RoomType::Puzzle,
            "yellow" => RoomType::Yellow,
            "blood" => RoomType::Blood,
            "fairy" => RoomType::Fairy,
            "entrance" => RoomType::Entrance,
            "trap" => RoomType::Trap,
            _ => RoomType::Normal,
        }
    }
}

#[derive(Debug)]
pub struct RoomData {
    pub name: String,
    pub shape: RoomShape,
    pub room_type: RoomType,
    pub bottom: i32,
    pub width: i32,
    pub length: i32,
    pub height: i32,
    pub block_data: Vec<Blocks>,
}

impl RoomData {
    pub fn from_raw_json(raw_data: &str) -> RoomData {
        let json_data: Value = serde_json::from_str(raw_data).unwrap();

        let name = json_data["name"].as_str().unwrap().to_string();
        let shape = RoomShape::from_str(json_data["shape"].as_str().unwrap());
        let room_type = RoomType::from_str(json_data["type"].as_str().unwrap());
        let bottom = json_data["bottom"].as_number().unwrap().as_u64().unwrap() as i32;
        let width = json_data["width"].as_number().unwrap().as_u64().unwrap() as i32;
        let length = json_data["length"].as_number().unwrap().as_u64().unwrap() as i32;
        let height = json_data["height"].as_number().unwrap().as_u64().unwrap() as i32;

        let hex_data = json_data["block_data"].as_str().unwrap();

        let mut block_data: Vec<Blocks> = Vec::new();

        for i in (0..hex_data.len()).step_by(4) {
            let hex_str = hex_data.get(i..i+4).unwrap();
    
            let num = u16::from_str_radix(hex_str, 16).unwrap();
            let block = Blocks::from_block_state_id(num);
            
            block_data.push(block)
        }

        RoomData {
            name,
            shape,
            room_type,
            bottom,
            width,
            length,
            height,
            block_data,
        }
    }

    pub fn dummy() -> RoomData {
        RoomData {
            name: String::from("Dummy"),
            shape: RoomShape::OneByOne,
            room_type: RoomType::Normal,
            bottom: 68,
            width: 31,
            length: 31,
            height: 30,
            block_data: vec![]
        }
    }
}