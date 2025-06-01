use crate::net::packets::packet_write::PacketWrite;

#[derive(Clone, Debug)]
pub struct Metadata {
    pub(crate) base_metadata: BaseMetadata,
    pub(crate) entity_metadata: EntityMetadata,
}

impl PacketWrite for Metadata {
    fn write(&self, buf: &mut Vec<u8>) {
        self.base_metadata.write_to_buffer(buf);
        self.entity_metadata.write_to_buffer(buf);
    }
}

#[derive(Clone, Debug)]
pub struct BaseMetadata {
    pub(crate) name: String,
}

impl BaseMetadata {
    pub fn write_to_buffer(&self, buf: &mut Vec<u8>) {
        buf.push((4 << 5 & 255) as u8);
        self.name.write(buf)
    }
}

crate::metadata! {
    Player,
    Zombie {
        is_child: 12; bool,
        is_villager: 13; bool,
        is_converting: 14; bool
    }
}


#[macro_export]
macro_rules! metadata {
    ($($name:ident $({$($field:ident: $id:expr; $ty:tt), *$(,)?})?),* $(,)?) => {
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