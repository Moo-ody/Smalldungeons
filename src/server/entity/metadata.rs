use enum_dispatch::enum_dispatch;
use std::fmt::Debug;

/// Type representing Minecraft's [Data Watcher](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/entity/DataWatcher.java). 
/// Renamed to Metadata because thats pretty much exactly what it is and its more clear.
/// 
/// # Example:
/// ```
/// pub struct WatchedStruct {
///     watched_boolean: IsChild,
///     watched_int: Ticks,
///     watched_float: Health
/// }
/// 
/// meta_data!(IsChild, bool, 12);
/// meta_data!(Ticks, i32, 13);
/// meta_data!(Health, f32, 14);
/// meta_data_impl!(WatchedStruct, watched_boolean,  watched_int, watched_float;
/// ```
///
/// this should be replaced with an enum requiring registering metadata types at some point
/// since the boxed trait stuff is slow and unnecessary.
pub type Metadata = Vec<Box<dyn MetadataEntry + Send + Sync>>;

pub trait MetadataEntry: Debug + Send + Sync {
    fn write_to_buffer(&self, buf: &mut Vec<u8>) {}
}

#[enum_dispatch]
pub trait MetadataImpl {
    fn create_meta_data(&self) -> Metadata;
}

#[macro_export]
macro_rules! meta_data {
    ($name:ident, $typ:tt, $id:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name($typ);
        
        impl MetadataEntry for $name {
            fn write_to_buffer(&self, buf: &mut Vec<u8>) {
                buf.push(((crate::type_to_id!($typ) << 5 | $id & 31) & 255) as u8);
                crate::net::packets::packet_write::PacketWrite::write(&self.0, buf);
            }
        }
    };
}

/// macro handling meta data types. 
/// This is missing a few types still.
#[macro_export]
macro_rules! type_to_id {
    (bool) => { u8::from(0) }; // this needs the from stuff otherwise it cries 
    (i16) => { u8::from(1) };
    (i32) => { u8::from(2) };
    (f32) => { u8::from(3) };
    (String) => { u8::from(4) };
    (ItemStack) => { u8::from(5) };
    // currently missing blockpos and rotation (both are vec3fs internally though so maybe just put that here?)

    // Catch-all for unsupported types
    ($other:ty) => {
        compile_error!(concat!("Unsupported type: ", stringify!($other)))
    };
}


#[macro_export]
macro_rules! meta_data_impl {
    ($name:ty$(, $value:ident )* $(,)?) => {
        impl MetadataImpl for $name {
            fn create_meta_data(&self) -> Metadata {
                vec![$(Box::new(self.$value.clone()),)*]
            }
        }
    }
}