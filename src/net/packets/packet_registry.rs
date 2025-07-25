
pub trait IdentifiedPacket {
    const PACKET_ID: i32;
}

#[macro_export]
macro_rules! register_packets {
    ($($packet:ty = $id:expr);* $(;)?) => {
        $(
            impl IdentifiedPacket for $packet {
                const PACKET_ID: i32 = $id;
            }
        )*
    };
}