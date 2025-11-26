use bitflags::bitflags;

bitflags::bitflags! {
    /// Vanilla player "skin parts" bitmask.
    /// 0x01 Cape, 0x02 Jacket, 0x04 L-Sleeve, 0x08 R-Sleeve,
    /// 0x10 L-Pants, 0x20 R-Pants, 0x40 Hat
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct SkinParts: u8 {
        const CAPE         = 0x01;
        const JACKET       = 0x02;
        const LEFT_SLEEVE  = 0x04;
        const RIGHT_SLEEVE = 0x08;
        const LEFT_PANTS   = 0x10;
        const RIGHT_PANTS  = 0x20;
        const HAT          = 0x40;
    }
}

impl Default for SkinParts {
    fn default() -> Self {
        // Default to showing all cosmetic layers except cape
        SkinParts::JACKET | SkinParts::LEFT_SLEEVE | SkinParts::RIGHT_SLEEVE | 
        SkinParts::LEFT_PANTS | SkinParts::RIGHT_PANTS | SkinParts::HAT
    }
}
