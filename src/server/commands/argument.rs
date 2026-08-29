#[derive(Debug)]
pub struct Argument {
    pub name: &'static str,
    pub required: bool,
    pub completions: Vec<String>,
}

impl Argument {
    pub fn new(name: &'static str, required: bool, completions: Vec<String>) -> Self {
        Self {
            name,
            required,
            completions,
        }
    }
}