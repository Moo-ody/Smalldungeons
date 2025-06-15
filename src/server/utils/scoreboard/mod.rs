pub mod scoreboard;

use crate::net::packets::packet_write::PacketWrite;

/// [String] with a character size limit of N.
/// All characters after that size will be truncated.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SizedString<const N: usize> {
    value: String,
}

impl<const N: usize> SizedString<N> {
    pub fn truncated(text: &str) -> Self {
        let truncated = text.chars().take(N).collect::<String>();
        Self { value: truncated }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl<const N: usize> PacketWrite for SizedString<N> {
    fn write(&self, writer: &mut Vec<u8>) {
        self.value.write(writer);
    }
}

impl<const N: usize> From<&str> for SizedString<N> {
    fn from(text: &str) -> Self {
        Self::truncated(text)
    }
}

impl<const N: usize> From<String> for SizedString<N> {
    fn from(text: String) -> Self {
        Self::truncated(&text)
    }
}