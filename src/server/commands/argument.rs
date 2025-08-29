#[derive(Debug)]
pub struct Argument {
    pub name: &'static str,
    pub completions: Vec<String>,
}