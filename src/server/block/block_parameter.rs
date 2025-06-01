

#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Axis {
    Y, // y is first for whatever reason
    X,
    Z,
    None
}