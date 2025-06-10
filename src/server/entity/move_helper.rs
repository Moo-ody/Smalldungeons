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

    pub fn update(&mut self, entity: &mut Entity) -> anyhow::Result<()> {
        if !self.update {
            return Ok(());
        };
        self.update = false;
        let x = self.pos.x - entity.pos.x;
        let z = self.pos.z - entity.pos.z;
        let i = entity.aabb.min_y.round();
        let y = self.pos.y - i;

        let g = z.mul_add(z, x.mul_add(x, y * y));

        if g < 2.500000277905201e-7 {
            return Ok(()); // this might need to error out idrk
        }

        let yaw = x.atan2(z).to_degrees() as f32 - 90.0;
        entity.yaw = limit_angle(entity.yaw, yaw, 30.0);
        //todo: entity move speed stuff
        if y > 0.0 && x * x + z * z < 1.0 {
            //todo: jump helper jump
        }
        Ok(())
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