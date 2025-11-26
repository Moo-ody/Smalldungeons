use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum MCColors {
    Black,
    DarkBlue,
    DarkGreen,
    DarkCyan,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Reset,
}

impl Serialize for MCColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let color_str = match self {
            MCColors::Black => "black",
            MCColors::DarkBlue => "dark_blue",
            MCColors::DarkGreen => "dark_green",
            MCColors::DarkCyan => "dark_cyan",
            MCColors::DarkRed => "dark_red",
            MCColors::DarkPurple => "dark_purple",
            MCColors::Gold => "gold",
            MCColors::Gray => "gray",
            MCColors::DarkGray => "dark_gray",
            MCColors::Blue => "blue",
            MCColors::Green => "green",
            MCColors::Aqua => "aqua",
            MCColors::Red => "red",
            MCColors::LightPurple => "light_purple",
            MCColors::Yellow => "yellow",
            MCColors::White => "white",
            MCColors::Reset => "reset",
        };
        serializer.serialize_str(color_str)
    }
}

impl<'de> Deserialize<'de> for MCColors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "black" => Ok(MCColors::Black),
            "dark_blue" => Ok(MCColors::DarkBlue),
            "dark_green" => Ok(MCColors::DarkGreen),
            "dark_cyan" => Ok(MCColors::DarkCyan),
            "dark_red" => Ok(MCColors::DarkRed),
            "dark_purple" => Ok(MCColors::DarkPurple),
            "gold" => Ok(MCColors::Gold),
            "gray" => Ok(MCColors::Gray),
            "dark_gray" => Ok(MCColors::DarkGray),
            "blue" => Ok(MCColors::Blue),
            "green" => Ok(MCColors::Green),
            "aqua" => Ok(MCColors::Aqua),
            "red" => Ok(MCColors::Red),
            "light_purple" => Ok(MCColors::LightPurple),
            "yellow" => Ok(MCColors::Yellow),
            "white" => Ok(MCColors::White),
            "reset" => Ok(MCColors::Reset),
            _ => Err(serde::de::Error::custom(format!("Unknown color: {}", s))),
        }
    }
}