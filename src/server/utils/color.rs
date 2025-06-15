use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Black,
    Dark_Blue,
    Dark_Green,
    Dark_Cyan,
    Dark_Red,
    Dark_Purple,
    Gold,
    Gray,
    Dark_Gray,
    Blue,
    Green,
    Aqua,
    Red,
    Light_Purple,
    Yellow,
    White,
    Reset,
}