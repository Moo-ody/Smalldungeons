use crate::dungeon::{Dungeon, OpenDoorTask};
use crate::net::packets::client_bound::entity::destroy_entities::DestroyEntities;
use crate::net::packets::client_bound::entity::entity_attach::EntityAttach;
use crate::net::packets::client_bound::spawn_object::PacketSpawnObject;
use crate::server::block::block_parameter::Axis;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::utils::dvec3::DVec3;
use crate::server::world;
use crate::server::world::World;
use rand::prelude::IndexedRandom;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DoorType {
    NORMAL,
    ENTRANCE,
    WITHER,
    BLOOD,
}

impl DoorType {
    const fn get_block(&self) -> Blocks {
        match self {
            DoorType::NORMAL => Blocks::Air,
            DoorType::ENTRANCE => Blocks::SilverfishBlock { variant: 5 },
            DoorType::WITHER => Blocks::CoalBlock,
            DoorType::BLOOD => Blocks::StainedHardenedClay { color: 14 }
        }
    }
}

#[derive(Debug)]
pub struct Door {
    pub id: usize,
    pub x: i32,
    pub z: i32,

    pub direction: Axis,
    pub door_type: DoorType,
}

impl Door {

    pub fn load_into_world(
        &self,
        world: &mut World,
        // maybe somehow make this constant? idk
        door_blocks: &HashMap<DoorType, Vec<Vec<Blocks>>>,
    ) {
        // Area to fill with air
        let (dx, dz) = match self.direction {
            Axis::X => (3, 2),
            _ => (2, 3),
        };

        // Doors have a thick bedrock floor usually
        world.fill_blocks(
            Blocks::Bedrock,
            BlockPos { x: self.x - dx, y: 67, z: self.z - dz },
            BlockPos { x: self.x + dx, y: 66, z: self.z + dz },
        );

        // Might need to replace with a random palette of cobble, stone, gravel etc if we want to mimic hypixel FULLY, but this works fine.
        world.fill_blocks(
            Blocks::Stone { variant: 0 },
            BlockPos { x: self.x - (dz - 2) * 2, y: 68, z: self.z - (dx - 2) * 2 },
            BlockPos { x: self.x + (dz - 2) * 2, y: 68, z: self.z + (dx - 2) * 2 },
        );

        world.fill_blocks(
            Blocks::Air,
            BlockPos { x: self.x - dx, y: 69, z: self.z - dz },
            BlockPos { x: self.x + dx, y: 73, z: self.z + dz },
        );

        // Pretty much just to get a normal self from a wither one,
        // since wither doors are just normal doors with coal blocks.
        let door_type = match self.door_type {
            DoorType::BLOOD => DoorType::BLOOD,
            DoorType::ENTRANCE => DoorType::ENTRANCE,
            DoorType::WITHER | DoorType::NORMAL => DoorType::NORMAL,
        };

        let block_data = door_blocks.get(&door_type).unwrap();
        let mut rng = rand::rng();
        let chosen = block_data.choose(&mut rng).unwrap();
        let self_direction = self.direction.get_direction();

        for (i, block) in chosen.iter().enumerate() {
            let x = (i % 5) as i32;
            let z = ((i / 5) % 5) as i32;
            let y = (i / (5 * 5)) as i32;

            let bp = BlockPos { x: x - 2, y, z: z - 2 }.rotate(self_direction);

            let mut block_to_place = block.clone();
            block_to_place.rotate(self_direction);
            world.set_block_at(block_to_place, self.x + bp.x, 69 + bp.y, self.z + bp.z);
        }

        world.fill_blocks(
            self.door_type.get_block(),
            BlockPos { x: self.x - 1, y: 69, z: self.z - 1 },
            BlockPos { x: self.x + 1, y: 72, z: self.z + 1 },
        );
    }


    pub fn open_door(&self, world: &mut World) {
        if cfg!(debug_assertions) {
            assert_ne!(self.door_type, DoorType::NORMAL);
        }

        let mut entities = Vec::new();
        world::iterate_blocks(
            BlockPos { x: self.x - 1, y: 69, z: self.z - 1 },
            BlockPos { x: self.x + 1, y: 72, z: self.z + 1 },

            |x,y, z| {
                world.set_block_at(Blocks::Barrier, x, y, z);
                
                let id = world.spawn_entity(
                    DVec3::new(x as f64 + 0.5, y as f64, z as f64 + 0.5),
                    EntityVariant::Bat { hanging: false },
                    DoorEntityImpl {
                        block: self.door_type.get_block(),
                    }
                ).unwrap();
                
                entities.push(id);
            }
        );
        world.server_mut().dungeon.test.push(OpenDoorTask {
            ticks_left: 20,
            door_index: self.id,
            door_entity_ids: entities,
        });
    }
}

///
#[derive(Debug)]
pub struct DoorEntityImpl {
    pub block: Blocks,
}

/// offset so that the falling block riding the bat
pub const DOOR_ENTITY_OFFSET: f64 = 0.65;

impl EntityImpl for DoorEntityImpl {

    fn spawn(&mut self, entity: &mut Entity) {
        let world = entity.world_mut();
        let entity_id = world.new_entity_id();

        let object_data = {
            let block_state_id = self.block.get_block_state_id() as i32;
            let block_id = block_state_id >> 4;
            let metadata = block_state_id & 0b1111;
            block_id | (metadata << 12)
        };

        let falling_block_pos = entity.position.clone();
        entity.position.y -= DOOR_ENTITY_OFFSET;

        let spawn_packet = PacketSpawnObject::new(
            entity_id,
            EntityVariant::FallingBlock,
            falling_block_pos,
            DVec3::ZERO,
            0.0,
            0.0,
            object_data
        );

        let attach_packet = EntityAttach {
            entity_id,
            vehicle_id: entity.id,
            leash: false,
        };

        for player in world.players.values() {
            player.send_packet(spawn_packet.clone()).unwrap();
            player.send_packet(attach_packet.clone()).unwrap();
        }
    }

    fn tick(&mut self, entity: &mut Entity) {
        // probably when i get real values, destroy entity in here
        // maybe add ease in interpolation to make improve animation
        entity.position.y -= 0.25;
        println!("entity position y {}", entity.position.y)
    }

    fn despawn(&mut self, entity: &mut Entity) {
        let destroy_packet = DestroyEntities {
            entity_ids: vec![entity.id + 1],
        };
        for player in entity.world_mut().players.values() {
            player.send_packet(destroy_packet.clone()).unwrap();
        }
    }
}