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

/// Hypixel Mod API channel for the location packet. Registered under the `hyevent:` prefix
/// (not `hypixel:`) since it's an event packet, not a request/response one - confirmed via
/// `HypixelModAPI.registerEventPackets` in the bundled mod-api jar's bytecode. The old
/// `hypixel:location` string wasn't a registered identifier at all, so the client's packet
/// registry silently dropped every location packet this server sent (see
/// `ForgeModAPI$HypixelPacketHandler.channelRead0`: unregistered identifiers are ignored,
/// not errored), meaning `SBInfo.mode`/`serverType` never got set and Skytils' dungeon
/// detection never fired via this path.
pub const HYPIXEL_LOCATION_CHANNEL: &str = "hyevent:location";

const LOCATION_PACKET_VERSION: u8 = 1;

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

/// Every Hypixel Mod API packet (hello, location, etc.) is dispatched through
/// `HypixelModAPI.handle(String identifier, PacketSerializer)` client-side, which reads a
/// leading `success` boolean *before* handing off to the packet's own `read()` - `true` means
/// "here's the packet", `false` means "here's an error reason VarInt instead". Omitting this
/// byte (as this file previously did) shifts every subsequent field by one: for the location
/// packet specifically, the real version VarInt ends up read from the middle of the
/// server-name string, fails `isExpectedVersion()`, and the whole packet is silently dropped
/// before Skytils' handler ever sees it - which is why fixing just the channel identifier
/// wasn't enough on its own.
fn write_success_prefix(buf: &mut Vec<u8>) {
    buf.push(1);
}

/// Builds the payload for ClientboundHelloPacket (version 1).
/// The client sets onHypixel = true when it receives this; send it before location.
/// Format: success (1 byte, always true here) | environment (VarInt: 0=PRODUCTION, 1=BETA, 2=TEST).
pub fn build_hello_payload() -> Vec<u8> {
    let mut buf = Vec::new();
    write_success_prefix(&mut buf);
    write_var_int(&mut buf, HELLO_ENVIRONMENT_PRODUCTION);
    buf
}

const HELLO_ENVIRONMENT_PRODUCTION: i32 = 0;

/// Builds the payload for ClientboundLocationPacket (version 1).
/// Format: success (1 byte) | version (VarInt) | serverName (string) | serverType? | lobbyName? | mode? | map?
/// Optional fields are: 1 byte (0/1) then if 1, Minecraft string.
pub fn build_location_payload(
    server_name: &str,
    server_type: Option<&str>,
    lobby_name: Option<&str>,
    mode: Option<&str>,
    map: Option<&str>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_success_prefix(&mut buf);
    write_var_int(&mut buf, LOCATION_PACKET_VERSION as i32);
    write_mc_string(&mut buf, server_name);
    write_optional_string(&mut buf, server_type);
    write_optional_string(&mut buf, lobby_name);
    write_optional_string(&mut buf, mode);
    write_optional_string(&mut buf, map);
    buf
}
