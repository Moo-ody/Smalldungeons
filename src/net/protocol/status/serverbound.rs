use crate::net::client::Client;
use crate::net::packets::packet::{ProcessContext, ProcessPacket};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::status::clientbound::{StatusPong, StatusResponse};
use crate::{register_serverbound_packets, STATUS_RESPONSE_JSON};
use blocks::packet_deserializable;

register_serverbound_packets! {
    Status;
    StatusRequest = 0x00;
    StatusPing = 0x01;
}

packet_deserializable! {
    pub struct StatusRequest;
}

impl ProcessPacket for StatusRequest {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        let mut packet_buffer = PacketBuffer::new();
        packet_buffer.write_packet(&StatusResponse {
            status: STATUS_RESPONSE_JSON.parse()?,
        });
        context.network_thread_tx.send(packet_buffer.get_packet_message(&client.client_id))?;
        Ok(())
    }
}

packet_deserializable! {
    pub struct StatusPing {
        pub client_time: i64,
    }
}

impl ProcessPacket for StatusPing {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        let mut packet_buffer = PacketBuffer::new();
        packet_buffer.write_packet(&StatusPong {
            client_time: self.client_time,
        });
        context.network_thread_tx.send(packet_buffer.get_packet_message(&client.client_id))?;
        Ok(())
    }
}

