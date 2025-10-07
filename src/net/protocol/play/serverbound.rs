use crate::net::client::Client;
use crate::net::packets::packet::ProcessPacket;
use crate::net::packets::packet_deserialize::PacketDeserializable;
use crate::net::protocol::play::serverbound::ClientStatus::{OpenInventory, PerformRespawn, RequestStats};
use crate::net::var_int::VarInt;
use crate::register_serverbound_packets;
use crate::server::block::block_position::BlockPos;
use crate::server::items::item_stack::ItemStack;
use crate::server::utils::fvec3::FVec3;
use crate::server::utils::sized_string::SizedString;
use anyhow::bail;
use blocks::packet_deserializable;
use bytes::BytesMut;

register_serverbound_packets! {
    Play;
    KeepAlive = 0x00;
    ChatMessage = 0x01;
    UseEntity = 0x02;
    PlayerUpdate = 0x03;
    PlayerPosition = 0x04;
    PlayerLook = 0x05;
    PlayerPositionLook = 0x06;
    PlayerDigging = 0x07;
    PlayerBlockPlacement = 0x08;
    HeldItemChange = 0x09;
    ArmSwing = 0x0a;
    PlayerAction = 0x0b;
    // SteerVehicle = 0x0c;
    CloseWindow = 0x0d;
    ClickWindow = 0x0e;
    ConfirmTransaction = 0x0f;
    // CreativeInventoryAction = 0x10;
    // EnchantItem = 0x11;
    // SetSign = 0x12;
    // ClientAbilities = 0x13;
    TabComplete = 0x14;
    ClientSettings = 0x15;
    ClientStatus = 0x16;
    // CustomPayload = 0x17;
    // SpectateTeleport = 0x18;
    // ResourcePackStatus = 0x19;
}

packet_deserializable! {
    pub struct KeepAlive {
        pub id: i32,
    }
}

packet_deserializable! {
    pub struct ChatMessage {
        pub message: SizedString<100>
    }
}

packet_deserializable! {
    #[derive(Debug, PartialEq)]
    pub enum EntityInteractionType {
        Interact,
        Attack,
        InteractAt, // used in armor stands
    }
}

pub struct UseEntity {
    pub entity_id: VarInt,
    pub action: EntityInteractionType,
    pub hit_vec: Option<FVec3>
}

impl PacketDeserializable for UseEntity {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        let entity_id: VarInt = PacketDeserializable::read(buffer)?;
        let action: EntityInteractionType = PacketDeserializable::read(buffer)?;
        let hit_vec = if action == EntityInteractionType::InteractAt { 
            Some(FVec3::new(
                PacketDeserializable::read(buffer)?,
                PacketDeserializable::read(buffer)?,
                PacketDeserializable::read(buffer)?,
            ))
        } else {
            None
        };
        Ok(Self {
            entity_id,
            action,
            hit_vec, 
        })
    }
}

packet_deserializable! {
    pub struct PlayerUpdate {
         pub on_ground: bool
    }
}

packet_deserializable! {
    pub struct PlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub on_ground: bool,
    }    
}

packet_deserializable! {
    pub struct PlayerLook {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }
}

packet_deserializable! {
    pub struct PlayerPositionLook {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }
}

packet_deserializable! {
    pub enum PlayerDiggingAction {
        StartDestroyBlock,
        AbortDestroyBlock,
        FinishDestroyBlock,
        DropAllItem, // < probably won't need these
        DropItem,    // <
        ReleaseUseItem // bow
    }
}


packet_deserializable! {
    pub struct PlayerDigging {
        pub action: PlayerDiggingAction,
        pub position: BlockPos,
        pub direction: i8,
    }
}

packet_deserializable! {
    pub struct PlayerBlockPlacement {
        pub position: BlockPos,
        pub placed_direction: i8,
        pub item_stack: Option<ItemStack>,
        pub facing_x: i8,
        pub facing_y: i8,
        pub facing_z: i8,
    }
}

packet_deserializable! {
    pub struct HeldItemChange {
        // for some reason this is a short
        pub slot_id: i16,
    }
}

packet_deserializable! {
    pub struct ArmSwing;
}

pub enum PlayerActionType {
    StartSneaking,
    StopSneaking,
    StopSleeping,
    StartSprinting,
    StopSprinting,
    RidingJump,
    OpenInventory,
}

impl PacketDeserializable for PlayerActionType {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        let var_int: VarInt = PacketDeserializable::read(buffer)?;
        Ok({
            match var_int.0 {
                0 => PlayerActionType::StartSneaking,
                1 => PlayerActionType::StopSneaking,
                2 => PlayerActionType::StopSleeping,
                3 => PlayerActionType::StartSprinting,
                4 => PlayerActionType::StopSprinting,
                5 => PlayerActionType::RidingJump,
                6 => PlayerActionType::OpenInventory,
                _ => bail!("failed to read player digging action, invalid index: {}", var_int.0)
            }
        })
    }
}

packet_deserializable! {
    pub struct PlayerAction {
        pub entity_id: VarInt,
        pub action: PlayerActionType,
        pub data: VarInt,
    }
}


packet_deserializable! {
    pub struct CloseWindow {
        pub window_id: u8
    }
}

packet_deserializable! {
    #[derive(PartialEq)]
pub enum ClickMode {
        NormalClick,
        ShiftClick,
        NumberKey,
        MiddleClick,
        Drop,
        Drag,
        DoubleClick,
    }
}

packet_deserializable! {
    pub struct ClickWindow {
        pub window_id: i8,
        pub slot_id: i16,
        pub used_button: i8,
        pub action_number: i16,
        pub mode: ClickMode,
        pub clicked_item: Option<ItemStack>,
    }
}

packet_deserializable! {
    pub struct ConfirmTransaction {
        pub window_id: i8,
        pub action_number: i16,
        pub accepted: bool,
    }
}

pub struct TabComplete {
    pub message: String,
    pub target_block: Option<BlockPos>
}

impl PacketDeserializable for TabComplete {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            message: {
                let msg: SizedString<32767> = SizedString::read(buffer)?;
                msg.into_owned()
            },
            target_block: {
                if u8::read(buffer)? != 0 {
                    Some(BlockPos::read(buffer)?)
                } else { 
                    None 
                }
            },
        })
    }
}

packet_deserializable! {
    pub struct ClientSettings {
        pub lang: SizedString<7>,
        pub view_distance: i8,
        pub chat_mode: i8,
        pub chat_colors: bool,
        pub skin_parts: u8,
    }
}

pub enum ClientStatus {
    PerformRespawn,
    RequestStats,
    OpenInventory,
}

impl PacketDeserializable for ClientStatus {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        let var_int: VarInt = PacketDeserializable::read(buffer)?;
        Ok({
            match var_int.0 {
                0 => PerformRespawn,
                1 => RequestStats,
                2 => OpenInventory,
                _ => bail!("failed to read client status, invalid index: {}", var_int.0)
            }
        })
    }
}