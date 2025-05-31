use crate::server::entity::ai::{TaskData, TaskType};
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use rand::random_range;

impl TaskData {
    pub fn default(task_type: TaskType) -> Self {
        match task_type {
            TaskType::WatchClosest => Self::WatchClosest {
                closest: None,
                max_distance: 100.0,
                look_time: 0,
                chance: 0.5,
                watched_entity_type: EntityType::Player,
            }
        }
    }

    pub fn should_run(&mut self, executing: &mut Entity, world: &mut World) -> bool {
        match self {
            Self::WatchClosest {
                closest,
                max_distance,
                look_time: _,
                chance,
                watched_entity_type,
            } => {
                if random_range(0.0..1.0) >= *chance {
                    return false;
                };

                if let Some(target) = executing.ai_target {
                    *closest = Some(target);
                }

                *closest = if *watched_entity_type == EntityType::Player {
                    world.get_closest_player(&executing.pos, *max_distance).map(|e| e.entity_id)
                } else { None /* todo: get closest entity within aabb using entity's bounding box epanded by max dist*/ };

                closest.is_some()
            }
        }
    }

    pub fn keep_executing(&mut self, executing: &mut Entity, world: &mut World) -> bool {
        match self {
            Self::WatchClosest {
                closest: _,
                max_distance,
                look_time,
                ..
            } => {
                if let Some(target) = executing.ai_target.and_then(|target_id| { world.entities.get(&target_id) }) {
                    if !target.is_alive() {
                        return false;
                    }

                    if executing.pos.distance_squared(&target.pos) > f64::from(*max_distance * *max_distance) {
                        return false;
                    }
                }

                look_time > &mut 0
            }
        }
    }

    pub fn start_executing(&mut self, executing: &mut Entity, world: &mut World) {
        match self {
            Self::WatchClosest { closest: _, max_distance: _, look_time, .. } => { *look_time = 40 + random_range(0..40) }
        }
    }

    pub fn update(&mut self, executing: &mut Entity, world: &mut World) {
        match self {
            Self::WatchClosest {
                closest,
                max_distance: _,
                look_time,
                chance: _,
                watched_entity_type: _,
            } => {
                if let Some(target) = closest.and_then(|target_id| { world.entities.get(&target_id) }) {
                    executing.look_helper.set_pos(target.pos + Vec3f::from_y(1.64), 10.0, 10.0)
                }
                *look_time -= 1;
            }
        }
    }

    pub fn should_continue(&mut self, executing: &mut Entity, world: &mut World) -> bool {
        match self {
            Self::WatchClosest { .. } => { self.keep_executing(executing, world) }
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::WatchClosest {
                closest, ..
            } => { *closest = None }
        }
    }
}