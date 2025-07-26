use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::var_int::write_var_int;
use crate::server::player::player::ClientId;

#[derive(Debug)]
pub struct PacketBuffer {
    pub buffer: Vec<u8>
}

impl PacketBuffer {
    
    pub fn new() -> Self {
        Self {
            buffer: Vec::new()
        }
    }

    pub fn write_packet<P : IdentifiedPacket + PacketSerializable>(&mut self, packet: &P) {
        // ideally, we wouldn't need to allocate this extra vec, 
        // but that would complicate creating packets by a lot, and we don't need the performance
        let mut payload: Vec<u8> = Vec::with_capacity(32);
        write_var_int(&mut payload, P::PACKET_ID);
        packet.write(&mut payload);
        write_var_int(&mut self.buffer, payload.len() as i32);
        self.buffer.extend(payload);
    }
    
    pub fn copy_from(&mut self, buf: &PacketBuffer) {
        self.buffer.extend(&buf.buffer)
    }

    /// gets a message for network thread to send the packets inside the buffer to the client.
    pub fn get_packet_message(&mut self, client_id: &ClientId) -> NetworkThreadMessage {
        // println!("buffer size {}", self.buffer.len());
        NetworkThreadMessage::SendPackets {
            client_id: *client_id,
            buffer: std::mem::replace(&mut self.buffer, Vec::new()),
        }
    }
}