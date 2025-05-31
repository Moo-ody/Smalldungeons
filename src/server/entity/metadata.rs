use std::fmt::Debug;


crate::meta_data! {
    all {
        Name, String, 2, METANAMEID,
    },
    zombie {
        IsChild, bool, 12, METACHILDID,
        IsVillager, bool, 13, METAVILLAGERID,
        IsConverting, bool, 14, METACONVERTINGID,
    },
}

/// macro to create a representation of  Minecraft's [Data Watcher](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/entity/DataWatcher.java) structure.
/// Renamed to Metadata because thats pretty much exactly what it is and its more clear.
///
/// maybe should be changed to per entity enum struct thingy with values? idk.
///
/// entity ident isnt used rn but i think it might be used later.
///
/// structure:
/// ```
/// entitytype {
///     Metadataentryname, type, id, idconstname,
/// }
/// ```
#[macro_export]
macro_rules! meta_data {
    {$($entity:ident {$($name:ident, $typ:tt, $id:expr, $id_const:ident),* $(,)?}),* $(,)?} => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub enum Metadata {
            $(
                $(
                    $name($typ),
                )*
            )*
        }

        $(
            $(
               pub const $id_const: i8 = $id;
            )*
        )*

        impl Metadata {
            #[inline]
            pub const fn get_id(&self) -> i8 {
                match self {
                    $(
                        $(
                            Self::$name(_) => $id_const,
                        )*
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
                            },
                        )*
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