use crate::server::entity::attributes::Attribute;
use crate::server::entity::entity::Entity;
use crate::server::entity::look_helper::wrap_to_180;
use crate::server::utils::vec3f::Vec3f;

#[derive(Clone, Debug)]
pub struct MoveHelper {
    pub pos: Vec3f,
    pub speed: f64,
    update: bool,
}

impl MoveHelper {
    pub fn from_pos(pos: Vec3f) -> Self {
        Self {
            pos,
            speed: 0.0,
            update: false,
        }
    }

    pub fn set_move_to(&mut self, pos: Vec3f, speed: f64) {
        self.pos = pos;
        self.speed = speed;
        self.update = true;
    }

    pub fn update(entity: &mut Entity) {
        // set move forward 0?
        if !entity.move_helper.update {
            return 
        };
        entity.move_helper.update = false;
        let x = entity.move_helper.pos.x - entity.pos.x;
        let z = entity.move_helper.pos.z - entity.pos.z;
        let y = entity.move_helper.pos.y - entity.aabb.min_y.round();;

        let g = z.mul_add(z, x.mul_add(x, y * y));

        if g < 2.500000277905201e-7 {
            return // this might need to error out idrk
        }

        let yaw = x.atan2(z).to_degrees() as f32 - 90.0;
        entity.yaw = limit_angle(entity.yaw, yaw, 30.0);
        entity.set_move((entity.move_helper.speed * entity.move_speed()) as f32);
        if y > 0.0 && x.mul_add(x, z * z) < 1.0 {
            //todo: jump helper jump
        }
    }
}


pub fn limit_angle(angle1: f32, angle2: f32, clamp: f32) -> f32 {
    let mut limited: f32;
    let wrapped = wrap_to_180(angle2 - angle1).clamp(-clamp, clamp);
    limited = angle1 + wrapped;
    if limited < 0.0 {
        limited += 360.0;
    } else if (limited >= 360.0) {
        limited -= 360.0;
    }
    limited
}