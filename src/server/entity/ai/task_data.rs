use crate::server::entity::ai::ai_enum::TaskType;
use crate::server::entity::entity_type::EntityType;
use crate::server::world::World;
use rand::random_range;

#[derive(Clone, Debug, PartialEq)]
pub enum TaskData {
    WatchClosest {
        closest: Option<i32>,
        max_distance: f32,
        look_time: i32,
        chance: f32,
        watched_entity_type: EntityType, // if entity enum is changed to copy this structure, this would just be its type reference.
    },
}

impl TaskData {
    pub fn default(task_type: TaskType) -> Self {
        match task_type {
            TaskType::WatchClosest => Self::WatchClosest {
                closest: None,
                max_distance: 100.0,
                look_time: 0,
                chance: 0.5,
                watched_entity_type: EntityType::Player,
            },
            _ => todo!()
        }
    }

    pub fn should_run(&mut self, executing_entity: &i32, world: &mut World) -> bool {
        match self {
            TaskData::WatchClosest {
                closest,
                max_distance,
                look_time: _,
                chance,
                watched_entity_type,
            } => {
                if random_range(0.0..1.0) >= *chance {
                    return false;
                };

                if let Some(target) = world.entities[executing_entity].ai_target {
                    *closest = Some(target);
                }

                *closest = if *watched_entity_type == EntityType::Player {
                    todo!() // get closest player
                } else { todo!() /* get closest entity within aabb using entity's bounding box epanded by max dist*/ };
                closest.is_some()
            }
        }
    }

    pub fn keep_executing(&mut self, executing_entity: &i32, world: &mut World) -> bool {
        match self {
            TaskData::WatchClosest {
                closest: _,
                max_distance,
                look_time,
                chance: _,
                watched_entity_type: _,
            } => {
                if let Some(executing) = world.entities.get(executing_entity) {
                    if let Some(target) = executing.ai_target.and_then(|target_id| { world.entities.get(&target_id) }) {
                        if !target.is_alive() {
                            return false;
                        }

                        if executing.pos.distance_squared(&target.pos) > f64::from(*max_distance * *max_distance) {
                            return false;
                        }
                    }
                }

                look_time > &mut 0
            }
        }
    }

    pub fn start_executing(&mut self, executing_entity: &i32, world: &mut World) {
        match self {
            TaskData::WatchClosest { closest: _, max_distance: _, look_time, chance: _, watched_entity_type: _, } => { *look_time = 40 + rand::random_range(0..40) }
        }
    }

    pub fn update(&mut self, executing_entity: &i32, world: &mut World) {
        match self {
            TaskData::WatchClosest {
                closest: _,
                max_distance: _,
                look_time,
                chance: _,
                watched_entity_type: _,
            } => {
                // set look position
                *look_time -= 1;
            }
        }
    }

    pub fn should_continue(&mut self, executing_entity: &i32, world: &mut World) -> bool {
        match self {
            TaskData::WatchClosest {
                closest: _,
                max_distance: _,
                look_time: _,
                chance: _,
                watched_entity_type: _,
            } => { self.keep_executing(executing_entity, world) }
        }
    }

    pub fn reset(&mut self) {
        todo!()
    }
}