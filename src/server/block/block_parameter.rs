

#[repr(u8)]
#[derive(PartialEq)]
pub enum Axis {
    Y, // y is first for whatever reason
    X,
    Z,
    None
}