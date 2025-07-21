#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionType {
    Clear,
    Solid,
    Water,
    Lava,
    Fence,
    Trapdoor,
    Open,
}