use crate::server::entity::entity::{Entity, EntityImpl, EntityId};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::utils::dvec3::DVec3;
use crate::server::player::player::Player;
use crate::net::protocol::play::clientbound::SpawnObject;
use crate::net::protocol::play::clientbound::PacketEntityMetadata;
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::net::packets::packet_buffer::PacketBuffer;

#[derive(Debug, Clone)]
pub struct ArmorStand {
    pub entity_id: u32,
    pub position: DVec3,
    pub custom_name: Option<String>,
    pub invisible: bool,
    pub always_show_name: bool,
    pub terminal_id: Option<u32>, // Links to terminal if this is a terminal stand
}

impl ArmorStand {
    pub fn new(entity_id: u32, position: DVec3) -> Self {
        Self {
            entity_id,
            position,
            custom_name: None,
            invisible: false, // Make armor stands visible
            always_show_name: true,
            terminal_id: None,
        }
    }

    pub fn with_terminal(mut self, terminal_id: u32) -> Self {
        self.terminal_id = Some(terminal_id);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.custom_name = Some(name);
        self
    }

    pub fn spawn_packet(&self) -> SpawnObject {
        SpawnObject {
            entity_id: VarInt(self.entity_id as i32),
            entity_variant: 30, // Armor Stand entity type
            x: self.position.x,
            y: self.position.y,
            z: self.position.z,
            yaw: 0.0,
            pitch: 0.0,
            data: 0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
        }
    }

    pub fn metadata_packet(&self) -> PacketEntityMetadata {
        let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
        metadata.is_invisible = self.invisible;
        metadata.custom_name = self.custom_name.clone();
        
        PacketEntityMetadata {
            entity_id: VarInt(self.entity_id as i32),
            metadata,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal_id.is_some()
    }

    pub fn get_terminal_id(&self) -> Option<u32> {
        self.terminal_id
    }
}

pub struct ArmorStandImpl {
    pub armor_stand: ArmorStand,
}

impl ArmorStandImpl {
    pub fn new(armor_stand: ArmorStand) -> Self {
        Self { armor_stand }
    }
}

impl EntityImpl for ArmorStandImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Send spawn packet
        let spawn_packet = self.armor_stand.spawn_packet();
        packet_buffer.write_packet(&spawn_packet);
        
        // Send metadata packet
        let metadata_packet = self.armor_stand.metadata_packet();
        packet_buffer.write_packet(&metadata_packet);
    }

    fn despawn(&mut self, _entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Send destroy entities packet
        use crate::net::protocol::play::clientbound::DestroyEntites;
        let destroy_packet = DestroyEntites {
            entities: vec![VarInt(self.armor_stand.entity_id as i32)],
        };
        packet_buffer.write_packet(&destroy_packet);
    }

    fn tick(&mut self, _entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        // Armor stands don't need to do anything on tick
    }

    fn interact(&mut self, _entity: &mut Entity, player: &mut Player, _interaction_type: &EntityInteractionType) {
        // Handle terminal interaction
        if let Some(terminal_id) = self.armor_stand.terminal_id {
            // TODO: Open terminal for the player
            println!("Player {} interacted with terminal {}", player.client_id, terminal_id);
        }
    }
}
