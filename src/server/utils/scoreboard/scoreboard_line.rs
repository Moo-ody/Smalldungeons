use crate::server::utils::scoreboard::sized_string::SizedString;

#[derive(Debug, Clone)]
pub struct ScoreboardLine {
    pub line: SizedString<32>,
    pub dirty: bool,
}

impl ScoreboardLine {
    pub fn new(line: impl Into<SizedString<32>>) -> Self {
        Self {
            line: line.into(),
            dirty: true,
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
