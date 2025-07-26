use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::var_int::{write_var_int, VarInt};
use crate::register_packets;
use crate::server::block::block_position::BlockPos;
use crate::server::entity::entity_metadata::EntityMetadata;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::attribute::AttributeMap;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use crate::server::utils::player_list::player_profile::PlayerData;
use crate::server::utils::sized_string::SizedString;
use blocks::packet_serializable;
use std::rc::Rc;

register_packets! {
    KeepAlive = 0x00;
    JoinGame<'_> = 0x01;
    Chat = 0x02;
    UpdateTime = 0x03;
    EntityEquipment = 0x04;
    // SpawnPosition = 0x05;
    // UpdateHealth = 0x06;
    // Respawn = 0x07;
    PositionLook = 0x08;
    // SetHotbarSlot = 0x09;
    // EntityUsedBed = 0x0a;
    // SwingAnimation = 0x0b
    
    CollectItem = 0x0d;
    SpawnObject = 0x0e;
    SpawnMob = 0x0f;
    // SpawnPainting = 0x10;
    // SpawnExperienceOrb = 0x11;
    EntityVelocity = 0x12;
    DestroyEntites = 0x13;
    // Entity => 0x14;
    EntityMove = 0x15;
    EntityRotate = 0x16;
    EntityMoveRotate = 0x17;
    EntityTeleport = 0x18;
    EntityYawRotate = 0x19;
    EntityStatus = 0x1a;
    EntityAttach = 0x1b;
    PacketEntityMetadata = 0x1c;
    AddEffect = 0x1d;
    RemoveEffect = 0x1e;
    // SetExperience 0x1f;
    EntityProperties = 0x20;
    ChunkData = 0x21;
    // MultiBlockChange = 0x22;
    BlockChange = 0x23;
    BlockAction = 0x24;
    // BlockBreakAnimation = 0x25;
    // ChunkDataBulk = 0x26;
    // Explosion = 0x27;
    // Effect = 0x28
    SoundEffect<'_> = 0x29;
    Particles = 0x2a;
    // ChangeGameState = 0x2b
    // SpawnGlobalEntity = 0x2c
    OpenWindow = 0x2d;
    CloseWindow = 0x2e;
    SetSlot = 0x2f;
    WindowItems = 0x30;
    // WindowProperty = 0x31;
    ConfirmTransaction = 0x32;
    // UpdateSign = 0x33;
    // Maps = 0x34;
    // UpdateBlockEntity = 0x35;
    // SignEditorOpen = 0x36;
    // Statistics = 0x37;
    PlayerListItem = 0x38;
    // PlayerAbilities = 0x39;
    // TabCompleteReply = 0x3a
    ScoreboardObjective<'_> = 0x3b;
    UpdateScore = 0x3c;
    DisplayScoreboard = 0x3d;
    Teams = 0x3e;
    CustomPayload<'_> = 0x3f;
    Disconnect = 0x40;
    // ServerDifficulty = 0x41;
    // CombatEvent = 0x42;
    // Camera = 0x43;
    // WorldBorder = 0x44;
    // Title => 0x45;
    // SetCompression = 0x46;
    PlayerListHeaderFooter = 0x47;
    // ResourcePackSend = 0x48;
    // EntityUpdateNBT = 0x49
}

packet_serializable! {
    pub struct JoinGame<'a> {
        pub entity_id: i32, // not VarInt,
        pub gamemode: u8,
        pub dimension: u8,
        pub difficulty: u8,
        pub max_players: u8,
        pub level_type: &'a str,
        pub reduced_debug_info: bool,
    }
}



packet_serializable! {
    pub struct KeepAlive {
        pub current_time: i32,
    }
}



packet_serializable! {
    pub struct Chat {
        pub component: ChatComponentText,
        pub chat_type: i8,
    }
}

packet_serializable! {
    pub struct UpdateTime {
        pub world_age: i64,
        pub world_time: i64,
    }
}

packet_serializable! {
    pub struct EntityEquipment {
        pub entity_id: VarInt,
        pub item_slot: i16,
        pub item_stack: Option<ItemStack>
    }
}

packet_serializable! {
    pub struct PositionLook {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: u8,
    }
}

packet_serializable! {
    pub struct CollectItem {
        pub item_entity_id: VarInt,
        pub entity_id: VarInt,
    }
}

pub struct SpawnObject {
    pub entity_id: VarInt,
    pub entity_variant: i8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
    pub data: i32,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}

impl PacketSerializable for SpawnObject {
    fn write(&self, buf: &mut Vec<u8>) {
        self.entity_id.write(buf);
        self.entity_variant.write(buf);
        self.x.write(buf);
        self.y.write(buf);
        self.z.write(buf);
        self.pitch.write(buf);
        self.yaw.write(buf);
        self.data.write(buf);
        if self.data > 0 {
            self.velocity_x.write(buf);
            self.velocity_y.write(buf);
            self.velocity_z.write(buf);
        }
    }
}

packet_serializable! {
    pub struct SpawnMob {
        pub entity_id: VarInt,
        pub entity_variant: i8,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub yaw: i8,
        pub pitch: i8,
        pub head_pitch: i8,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
        pub metadata: EntityMetadata,
    }
}

packet_serializable! {
    pub struct EntityVelocity {
        pub entity_id: VarInt,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }
}

packet_serializable! {
    pub struct DestroyEntites {
        pub entities: Vec<VarInt>
    }
}

packet_serializable! {
    pub struct EntityMove {
        pub entity_id: VarInt,
        pub pos_x: i8,
        pub pos_y: i8,
        pub pos_z: i8,
        pub on_ground: bool,
    }
}

packet_serializable! {
    pub struct EntityRotate {
        pub entity_id: VarInt,
        pub yaw: i8,
        pub pitch: i8,
        pub on_ground: bool,
    }
}

packet_serializable! {
    pub struct EntityMoveRotate {
        pub entity_id: VarInt,
        pub pos_x: i8,
        pub pos_y: i8,
        pub pos_z: i8,
        pub yaw: i8,
        pub pitch: i8,
        pub on_ground: bool,
    }
}

packet_serializable! {
    pub struct EntityTeleport {
        pub entity_id: i32 => &VarInt(self.entity_id),
        pub pos_x: f64 => &((self.pos_x * 32.0).floor() as i32),
        pub pos_y: f64 => &((self.pos_y * 32.0).floor() as i32),
        pub pos_z: f64 => &((self.pos_z * 32.0).floor() as i32),
        pub yaw: f32 => &((self.yaw * 256.0 / 360.0) as i32 as i8),
        pub pitch: f32 => &((self.pitch * 256.0 / 360.0) as i32 as i8),
        pub on_ground: bool,
    }
}

packet_serializable! {
    pub struct EntityYawRotate {
        pub entity_id: VarInt,
        pub yaw: i8,
    }
}

packet_serializable! {
    pub struct EntityProperties {
        pub entity_id: VarInt,
        pub properties: AttributeMap,
    }
}

packet_serializable! {
    pub struct EntityStatus {
        pub entity_id: VarInt,
        pub logic_op_code: i8, // better name?
    }
}

packet_serializable! {
    pub struct EntityAttach {
        pub entity_id: i32, // not VarInt for whatever reason
        pub vehicle_id: i32,
        pub leash: bool,
    }
}

packet_serializable! {
    pub struct PacketEntityMetadata {
        pub entity_id: VarInt,
        pub metadata: EntityMetadata,
    }
}

packet_serializable! {
    pub struct AddEffect {
        pub entity_id: VarInt,
        pub effect_id: u8,
        pub amplifier: i8,
        pub duration: VarInt,
        pub hide_particles: bool,
    }
}

packet_serializable! {
    pub struct RemoveEffect {
        pub entity_id: VarInt,
        pub effect_id: u8,
    }
}

packet_serializable! {
    pub struct ChunkData {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub is_new_chunk: bool,
        pub bitmask: u16,
        pub data: Vec<u8>,
    }
}

packet_serializable! {
    pub struct BlockChange {
        pub block_pos: BlockPos,
        pub block_state: u16 => &VarInt(self.block_state as i32),
    }
}

packet_serializable! {
    pub struct BlockAction {
        pub block_pos: BlockPos,
        pub event_id: u8,
        pub event_data: u8,
        pub block_id: u16 => &VarInt((self.block_id & 4095) as i32),
    }
}

packet_serializable! {
    pub struct SoundEffect<'a> {
        pub sound: &'a str,
        pub pos_x: f64 => &((self.pos_x * 8.0) as i32),
        pub pos_y: f64 => &((self.pos_y * 8.0) as i32),
        pub pos_z: f64 => &((self.pos_z * 8.0) as i32),
        pub volume: f32,
        pub pitch: f32 => &(f32::clamp(self.pitch * 63.0, 0.0, 255.0) as u8),
    }
}

packet_serializable! {
    pub struct Particles {
        pub particle_id: i32,
        pub long_distance: bool,
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub offset_x: f32,
        pub offset_y: f32,
        pub offset_z: f32,
        pub speed: f32,
        pub count: i32,
        // maybe figure out args,
        // not sure if we'll ever need them
    }
}

packet_serializable! {
    pub struct OpenWindow {
        pub window_id: i8,
        pub inventory_type: SizedString<32>,
        pub window_title: ChatComponentText,
        pub slot_count: u8,
    }
}

packet_serializable! {
    pub struct CloseWindow {
        pub window_id: i8,
    }
}

packet_serializable! {
    pub struct SetSlot {
        pub window_id: i8,
        pub slot: i16,
        pub item_stack: Option<ItemStack>,
    }
}

pub struct WindowItems {
    pub window_id: i8,
    pub items: Vec<Option<ItemStack>>,
}

// why couldnt mojang use var int for length :(
impl PacketSerializable for WindowItems {
    fn write(&self, buf: &mut Vec<u8>) {
        self.window_id.write(buf);
        (self.items.len() as i16).write(buf);
        for item in self.items.iter() {
            item.write(buf);
        }
    }
}

packet_serializable! {
    pub struct ConfirmTransaction {
        pub window_id: i8,
        pub action_number: i16,
        pub accepted: bool,
    }
}

pub struct ScoreboardObjective<'a> {
    pub objective_name: SizedString<16>,
    pub objective_value: SizedString<32>,
    pub render_type: &'a str,
    pub mode: i8,
}

impl<'a> PacketSerializable for ScoreboardObjective<'a> {
    fn write(&self, buf: &mut Vec<u8>) {
        self.objective_name.write(buf);
        self.mode.write(buf);

        const ADD_OBJECTIVE: i8 = 0;
        const UPDATE_NAME: i8 = 2;

        if self.mode == ADD_OBJECTIVE || self.mode == UPDATE_NAME {
            self.objective_value.write(buf);
            self.render_type.write(buf);
        }
    }
}

pub struct UpdateScore {
    pub name: SizedString<40>,
    pub objective: SizedString<16>,
    pub value: VarInt,
    pub action: VarInt,
}

impl PacketSerializable for UpdateScore {
    fn write(&self, buf: &mut Vec<u8>) {
        self.name.write(buf);
        self.action.write(buf);
        self.objective.write(buf);
        
        if self.action.0 == 0 { 
            self.value.write(buf);
        }
    }
}

packet_serializable! {
    pub struct DisplayScoreboard {
        pub position: i8,
        pub score_name: SizedString<16>
    }
}

pub struct Teams {
    pub name: SizedString<16>,
    pub display_name: SizedString<32>,
    pub prefix: SizedString<16>,
    pub suffix: SizedString<16>,
    pub name_tag_visibility: SizedString<32>,
    pub color: i8,
    pub players: Vec<SizedString<40>>,
    pub action: i8,
    pub friendly_flags: i8,
}

impl PacketSerializable for Teams {
    fn write(&self, buf: &mut Vec<u8>) {
        pub const CREATE_TEAM: i8 = 0;
        pub const REMOVE_TEAM: i8 = 1;
        pub const UPDATE_TEAM: i8 = 2;
        pub const ADD_PLAYER: i8 = 3;
        pub const REMOVE_PLAYER: i8 = 4;

        self.name.write(buf);
        self.action.write(buf);

        if self.action == CREATE_TEAM || self.action == UPDATE_TEAM {
            self.display_name.write(buf);
            self.prefix.write(buf);
            self.suffix.write(buf);
            self.friendly_flags.write(buf);
            self.name_tag_visibility.write(buf);
            self.color.write(buf);
        }

        if self.action == CREATE_TEAM || self.action == ADD_PLAYER || self.action == REMOVE_PLAYER {
            self.players.write(buf);
            // VarInt(self.players.len() as i32).write(buf);
            // for player in self.players.iter() {
            //     player.write(buf);
            // }
        }
    }
}

// todo test, this might not work
packet_serializable! {
    pub struct CustomPayload<'a> {
        pub channel: SizedString<20>,
        pub data: &'a [u8]
    }
}

packet_serializable! {
    pub struct Disconnect {
        pub reason: ChatComponentText
    }
}

packet_serializable! {
    pub struct PlayerListHeaderFooter {
        pub header: ChatComponentText,
        pub footer: ChatComponentText,
    }
}

pub struct PlayerListItem {
    pub action: VarInt,
    pub players: Rc<[PlayerData]>
}

impl PacketSerializable for PlayerListItem {
    fn write(&self, buf: &mut Vec<u8>) {
        self.action.write(buf);
        write_var_int(buf, self.players.len() as i32);

        const ADD_PLAYER: i32 = 0;
        const UPDATE_GAME_MODE: i32 = 1;
        const UPDATE_LATENCY: i32 = 2;
        const UPDATE_NAME: i32 = 3;
        const REMOVE_PLAYER: i32 = 4;

        for player in self.players.iter() {
            match self.action.0 {
                ADD_PLAYER => {
                    player.profile.id.write(buf);
                    player.profile.name.write(buf);

                    let ref properties = player.profile.properties;
                    write_var_int(buf, properties.len() as i32);
                    for (key, property) in properties.iter() {
                        key.write(buf);
                        property.value.write(buf);

                        if let Some(signature) = &property.signature {
                            true.write(buf);
                            signature.write(buf)
                        } else {
                            false.write(buf)
                        }
                    }
                    write_var_int(buf, player.game_mode.id());
                    write_var_int(buf, player.ping);
                }
                UPDATE_GAME_MODE => {
                    player.profile.id.write(buf);
                    write_var_int(buf, player.game_mode.id());
                }
                UPDATE_LATENCY => {
                    player.profile.id.write(buf);
                    write_var_int(buf, player.ping);
                }
                UPDATE_NAME | REMOVE_PLAYER => {
                    player.profile.id.write(buf);
                }
                _ => unreachable!()
            }
            if self.action.0 == ADD_PLAYER || self.action.0 == UPDATE_NAME {
                if let Some(name) = &player.display_name {
                    true.write(buf);
                    name.write(buf);
                } else {
                    false.write(buf);
                }
            }
        }
    }
}