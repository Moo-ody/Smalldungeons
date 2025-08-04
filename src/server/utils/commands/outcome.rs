use crate::server::utils::chat_component::chat_component_text::ChatComponentText;

/// this should be used when the error is on the user side, such as an invalid argument type
pub enum Outcome {
    Success,
    Failure(ChatComponentText),
}