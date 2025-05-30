use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::attributes::Attributes;
use crate::server::entity::entity_type::EntityType;
use crate::server::entity::look_helper::LookHelper;
use crate::server::entity::metadata::Metadata;
use crate::server::utils::aabb::AABB;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use std::mem::take;

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_id: i32,
    pub entity_type: EntityType,
    // pub entity_type_data: EntityTypeData,
    pub pos: Vec3f,
    pub motion: Vec3f,
    pub prev_pos: Vec3f,
    pub last_sent_pos: Vec3f, // TEMPORARY, this will and should be different for every player.
    pub last_sent_yaw: f32,
    pub last_sent_pitch: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub aabb: AABB,
    pub height: f32,
    pub width: f32,
    pub ticks_existed: u32,
    pub health: f32,

    pub metadata: Vec<Metadata>,

    pub attributes: Attributes,

    pub ai_tasks: Option<AiTasks>,
    pub ai_target: Option<i32>,

    pub look_helper: LookHelper,
}

impl Entity {
    pub fn create_at(entity_type: EntityType, pos: Vec3f, id: i32) -> Entity {
        Entity {
            entity_id: id,
            entity_type,
            pos,
            motion: Vec3f::new_empty(),
            prev_pos: pos,
            last_sent_pos: pos,
            last_sent_yaw: 0.0,
            last_sent_pitch: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            head_yaw: 0.0,
            aabb: AABB::new_empty(), // aabb should be determined by height and width, which are determined by entity type and certain entity properties like size.
            health: 20.0, // todo: replace by using max health attribute, add requirement for attributes. could also make max health a normal param instead since its required but well see how i want to implement that in the attribute packet.
            height: 0.0,
            width: 0.0,
            ticks_existed: 0,

            metadata: entity_type.metadata(),

            attributes: Attributes::new(),

            ai_tasks: entity_type.get_tasks(),
            ai_target: None,

            look_helper: LookHelper::from_pos(pos, 10.0, 10.0)
        }
    }
    
    pub fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.prev_pos = self.pos;
        self.pos.x = x;
        self.pos.y = y;
        self.pos.z = z;
    }

    pub fn is_alive(&self) -> bool {
        !self.health.is_nan() && self.health > 0.0
    }

    pub fn update(mut self, world: &mut World) -> Self {
        // i dont know where in vanilla this happens but its necessary for vanilla to handle the packet properly and it isnt in the packet handling section.
        // living update mods yaw/pitch stuff if it got an update but that doesnt happen via at least the watchclosest ai and it wouldnt even work for this.
        self.head_yaw = LookHelper::wrap_to_180(self.head_yaw);
        self.update_state(world)
    }

    pub fn update_state(mut self, world: &mut World) -> Self {
        // check despawn
        // sensing cache clear

        // target ai update

        if let Some(mut tasks) = take(&mut self.ai_tasks) {
            tasks.update(&mut self, world).unwrap_or_else(|err| println!("error updating entity ai tasks: {:?}", err));
            self.ai_tasks = Some(tasks);
        }
        // path navigation update

        // generic task update?

        // move helper update
        LookHelper::on_update_look(&mut self);
        // jump helper update
        self
    }
}