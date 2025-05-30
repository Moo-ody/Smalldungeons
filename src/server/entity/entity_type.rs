use crate::server::entity::metadata::Metadata;

crate::entity_type_registry! {
    Zombie: zombie,
    Player: player,
}

/// macro to register entity types. this should handle all the unique entity type functions and whatnot.
/// only has 2 right now, and nothing which may need parameters, but it should be supported. you can have optional
/// ones but clarifying that theyre optional in the macro itself is weird, so atm they should be implemented but returning none.
#[macro_export]
macro_rules! entity_type_registry {
    {$($name:ident: $path:ident),* $(,)?} => {
        #[derive(Clone, Debug, PartialEq, Eq, Copy)]
        pub enum EntityType {
            $(
                $name,
            )*
        }

        impl EntityType {
            pub fn get_id(&self) -> i8 {
                match self {
                    $(
                        Self::$name => crate::server::entity::r#impl::$path::ID
                    ),*
                }
            }

            pub fn get_tasks(&self) -> Option<crate::server::entity::ai::ai_tasks::AiTasks> {
                match self {
                    $(
                        Self::$name => crate::server::entity::r#impl::$path::ai_tasks()
                    ),*
                }
            }

            pub fn metadata(&self) -> Vec<Metadata> {
                match self {
                    $(
                        Self::$name => crate::server::entity::r#impl::$path::metadata()
                    ),*
                }
            }
        }
    };
}