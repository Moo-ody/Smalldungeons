use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::attributes::Attributes;
use crate::server::entity::entity_type::EntityType;
use crate::server::entity::metadata::Metadata;
use crate::server::utils::aabb::AABB;
use crate::server::utils::vec3f::Vec3f;

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_id: i32,
    pub entity_type: EntityType,
    // pub entity_type_data: EntityTypeData,
    pub pos: Vec3f,
    pub motion: Vec3f,
    pub prev_pos: Vec3f,
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
}

impl Entity {
    pub fn create_at(entity_type: EntityType, pos: Vec3f, id: i32) -> Entity {
        Entity {
            entity_id: id,
            entity_type: entity_type.clone(),
            pos: pos.clone(),
            motion: Vec3f::new_empty(),
            prev_pos: pos,
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
        }
    }
    
    pub fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.prev_pos = self.pos.clone();
        self.pos.x = x;
        self.pos.y = y;
        self.pos.z = z;
    }

    pub fn is_alive(&self) -> bool {
        !self.health.is_nan() && self.health > 0.0
    }
}