use std::fmt::Debug;


crate::meta_data! {
    zombie {
        IsChild, bool, 12,
        IsVillager, bool, 13,
        IsConverting, bool, 14,
    },
}

/// macro to create a representation of  Minecraft's [Data Watcher](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/entity/DataWatcher.java) structure.
/// Renamed to Metadata because thats pretty much exactly what it is and its more clear.
///
/// entity ident isnt used rn but i think it might be used later.
#[macro_export]
macro_rules! meta_data {
    {$($entity:ident {$($name:ident, $typ:tt, $id:expr),* $(,)?}),* $(,)?} => {
        #[derive(Debug, Clone)]
        pub enum Metadata {
            $(
                $(
                    $name($typ)
                ),*
            )*
        }
        impl Metadata {
            pub fn get_id(&self) -> u8 {
                match self {
                    $(
                        $(
                            Self::$name(_) => $id
                        ),*
                    )*
                }
            }

            pub fn write_to_buffer(&self, buf: &mut Vec<u8>) {
                match self {
                    $(
                        $(
                            Self::$name(val) => {
                                buf.push(((crate::type_to_id!($typ) << 5 | $id & 31) & 255) as u8);
                                crate::net::packets::packet_write::PacketWrite::write(val, buf)
                            }
                        ),*
                    )*
                }
            }
        }
    }
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