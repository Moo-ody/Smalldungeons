use crate::net::packets::packet_registry::IdentifiedPacket;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::var_int::write_var_int;

#[derive(Debug)]
pub struct PacketBuffer {
    pub buf: Vec<u8>
}


impl PacketBuffer {

    pub fn write_packet<P : IdentifiedPacket + PacketSerializable>(&mut self, packet: &P) {
        let mut payload: Vec<u8> = Vec::with_capacity(32);
        write_var_int(&mut payload, P::PACKET_ID);
        packet.write(&mut payload);
        write_var_int(&mut self.buf, payload.len() as i32);
        self.buf.extend(payload);
    }
    
    pub fn extend(&mut self, buf: &PacketBuffer) {
        self.buf.extend(&buf.buf)
    }

    pub fn test(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }

}