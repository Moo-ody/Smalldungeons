use crate::net::packets::packet_serialize::PacketSerializable;

/// [String] with a character size limit of N.
/// All characters after that size will be truncated.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SizedString<const N: usize>(String);

impl<const N: usize> SizedString<N> {
    pub fn truncated(text: &str) -> Self {
        Self(text.chars().take(N).collect::<String>())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

impl<const N: usize> PacketSerializable for SizedString<N> {
    fn write(&self, writer: &mut Vec<u8>) {
        self.0.write(writer);
    }
}