use blocks::block_macro;

block_macro! {
    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum Blocks {
        Air,
        Stone {
            variant: u8,
        },
        Grass,
        Dirt {
            variant: u8,
        },
    }
}