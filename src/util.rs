pub fn little_endian_combine(a: u8, b: u8) -> u16 {
    (a as u16) | (b as u16) << 8
}
pub fn combine(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
pub fn split(a: u16) -> (u8, u8) {
    (((a & 0xFF00) >> 8) as u8, (a & 0xFF) as u8)
}