use crate::net::packets::packet_write::PacketWrite;

/// representation of Minecraft's [Data Watcher](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/entity/DataWatcher.java) structure.
/// Renamed to Metadata because thats pretty much exactly what it is and its more clear.
///
/// has 2 parts base metadata and entity metadata. base metadata is for all shared metadata values.
/// entity metadata is for metadata unique to an entity, such as is_child for zombies.
#[derive(Clone, Debug)]
pub struct Metadata {
    pub base: BaseMetadata,
    pub entity: EntityMetadata,
}

crate::metadata! {
    Player,
    Zombie {
        is_child: bool = 12,
        is_villager: bool = 13,
        is_converting: bool = 14
    }
}

impl PacketWrite for Metadata {
    fn write(&self, buf: &mut Vec<u8>) {
        self.base.write_to_buffer(buf);
        self.entity.write_to_buffer(buf);
        buf.push(127)
    }
}

#[derive(Clone, Debug)]
pub struct BaseMetadata {
    // all metadata is optional but this being like this makes it not optional. Maybe use Option<type> if it becomes an issue?
    // could be turned into a macro to include the basemetadata write_to_buffer impl...
    pub name: String,
}

impl BaseMetadata {
    // this needs to be updated for every entry added to base metadata.
    pub fn write_to_buffer(&self, buf: &mut Vec<u8>) {
        buf.push(((4 << 5 | 2 & 31) & 255) as u8);
        self.name.write(buf)
    }
}

#[macro_export]
macro_rules! metadata {
    ($($name:ident $({$($field:ident: $ty:tt = $id:expr), *$(,)?})?),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub enum EntityMetadata {
            $(
                $name $({
                    $($field: $ty),*
                })?
            ),*
        }

        impl EntityMetadata {
            pub fn write_to_buffer(&self, buf: &mut Vec<u8>) {
                match self {
                    $(
                        Self::$name $({$($field),* })? => {
                            $(
                                $(
                                    buf.push(((crate::type_to_id!($ty) << 5 | $id & 31) & 255) as u8);
                                    crate::net::packets::packet_write::PacketWrite::write($field, buf);
                                )*
                            )?
                        }
                    ),*
                }
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