//! Helpers to send Hypixel Mod API plugin messages so client mods (e.g. Skytils, SBA)
//! can treat this server as Hypixel. Matches Hypixel's ClientboundHelloPacket and
//! ClientboundLocationPacket (version 1) format.
//!
//! Skytils: `HypixelPacketEvent.ReceiveEvent` + `ClientboundLocationPacket` populate
//! `SBInfo` (mode, server, serverType) and post `LocationChangeEvent`. Use `mode` values
//! that match `SkyblockIsland.byMode` (e.g. "dungeon", "hub", "dynamic") so
//! `SkyblockIsland.current` resolves.

use crate::net::var_int::write_var_int;

/// Hypixel Mod API channel for the hello packet. Must be sent first so the client
/// sets onHypixel = true and accepts other Hypixel packets.
pub const HYPIXEL_HELLO_CHANNEL: &str = "hypixel:hello";

/// Hypixel Mod API channel for the location packet.
pub const HYPIXEL_LOCATION_CHANNEL: &str = "hypixel:location";

const LOCATION_PACKET_VERSION: u8 = 1;
const HELLO_PACKET_VERSION: u8 = 1;

/// Writes a Minecraft-style string: VarInt(length) + UTF-8 bytes.
fn write_mc_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_var_int(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

/// Writes an optional string: 1 byte (0 = absent, 1 = present), then if present the string.
fn write_optional_string(buf: &mut Vec<u8>, opt: Option<&str>) {
    match opt {
        None => buf.push(0),
        Some(s) => {
            buf.push(1);
            write_mc_string(buf, s);
        }
    }
}

/// Builds the payload for ClientboundHelloPacket (version 1).
/// The client sets onHypixel = true when it receives this; send it before location.
/// Format: version (1 byte).
pub fn build_hello_payload() -> Vec<u8> {
    vec![HELLO_PACKET_VERSION]
}

/// Builds the payload for ClientboundLocationPacket (version 1).
/// Format: version (1 byte) | serverName (string) | serverType? | lobbyName? | mode? | map?
/// Optional fields are: 1 byte (0/1) then if 1, Minecraft string.
pub fn build_location_payload(
    server_name: &str,
    server_type: Option<&str>,
    lobby_name: Option<&str>,
    mode: Option<&str>,
    map: Option<&str>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(LOCATION_PACKET_VERSION);
    write_mc_string(&mut buf, server_name);
    write_optional_string(&mut buf, server_type);
    write_optional_string(&mut buf, lobby_name);
    write_optional_string(&mut buf, mode);
    write_optional_string(&mut buf, map);
    buf
}
