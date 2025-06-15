use crate::server::entity::entity::Entity;
use crate::server::utils::vec3f::Vec3f;

/// this is weird and probably bad, but ill worry about moving and fixing it later
#[derive(Clone, Debug)]
pub struct LookHelper {
    pos: Vec3f,
    delta_yaw: f32,
    delta_pitch: f32,
    rotating: bool,
}

impl LookHelper {
    pub fn from_pos(pos: Vec3f, delta_yaw: f32, delta_pitch: f32) -> Self {
        Self {
            pos,
            delta_yaw,
            delta_pitch,
            rotating: true,
        }
    }

    pub fn set_pos(&mut self, pos: Vec3f, delta_yaw: f32, delta_pitch: f32) {
        self.pos = pos;
        self.delta_yaw = delta_yaw;
        self.delta_pitch = delta_pitch;
        self.rotating = true;
    }

    pub fn on_update_look(entity: &mut Entity) {
        // cant use self here because of borrow checker, but eventually this whole logic can probably be in the entity struct impl anyways.
        let helper = &mut entity.look_helper;
        // entity.pitch = 0.0; // this is necessary for some reason
        if helper.rotating {
            helper.rotating = false;
            let offset_x = helper.pos.x - entity.pos.x;
            let offset_y = helper.pos.y - entity.pos.y + 1.62; // todo: replace with entity.eye_height
            let offset_z = helper.pos.z - entity.pos.z;
            let offset_xz = offset_x.hypot(offset_z);
            let yaw = (offset_z as f32).atan2(offset_x as f32).to_degrees() - 90.0;
            let pitch = -(offset_y as f32).atan2(offset_xz as f32).to_degrees();
            entity.pitch = Self::update_rotation(entity.pitch, pitch, helper.delta_pitch);
            entity.head_yaw = Self::update_rotation(entity.head_yaw, yaw, helper.delta_yaw);
        } else {
            // todo entity.render_yaw_offset
        }
        // todo check if no path then clamp i think? its weird.
    }

    pub fn update_rotation(current: f32, target: f32, delta: f32) -> f32 {
        current + wrap_to_180(target - current).clamp(-delta, delta)
    }
}

pub fn wrap_to_180(angle: f32) -> f32 {
    // im betting this can be made simpler but im just tryna copy vanilla for the time being.
    let mut angle = angle % 360.0;
    if angle >= 180.0 {
        angle -= 360.0;
    }
    if angle <= -180.0 {
        angle += 360.0;
    }
    angle
}