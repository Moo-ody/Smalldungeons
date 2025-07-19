use crate::server::old_entity::entity_type::EntityType;

pub mod task_data;
pub mod ai_tasks;
pub mod pathfinding;

crate::ai_tasks! {
    WatchClosest: 2 => {
        closest: Option<i32>,
        max_distance: f32,
        look_time: i32,
        chance: f32,
        watched_entity_type: EntityType
    }
}

impl TaskType {
    pub const fn is_compatible(&self, other: &Self) -> bool {
        (self.id() & other.id()) == 0
    }
}

/// macro to make defining ai tasks simpler.
/// 
/// structure:
/// ```
///name: bitmask => {
///     value: type,
///     otherval: type,
/// }
/// ```
/// 
/// data stuff must be defined in the [TaskData] impl in task_data.rs file.
#[macro_export]
macro_rules! ai_tasks {
    ($($name:ident: $id:expr => { $($value:ident: $typ:tt$(<$($inner:ty),*>)?),*}),* $(,)?) => {
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
        pub enum TaskType {
            $($name),*
        }
        
        impl TaskType {
            pub const fn id(&self) -> u8 {
                match self {
                    $(Self::$name => $id),*
                }
            }
        }
        
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum TaskData {
            $(
                $name {
                    $(
                        $value: $typ$(<$($inner),*>)?
                    ),*
                }
            ),*
        }
    }
}