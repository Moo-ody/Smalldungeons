#[derive(Debug)]
pub struct Argument {
    pub name: &'static str,
    pub completions: Vec<String>,
}

impl Argument {
    pub fn new(name: &'static str, _required: bool, completions: Vec<String>) -> Self {
        Self {
            name,
            completions,
        }
    }
}