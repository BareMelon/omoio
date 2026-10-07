//! Linux uses normal emulator windows. Global window placement and keyboard
//! capture are intentionally unavailable, including on Wayland.

pub fn hide(_window: isize) {}
pub fn show(_window: isize, _activate: bool) {}
pub fn focus(_window: isize) {}
pub fn place(_window: isize, _x: i32, _y: i32, _width: i32, _height: i32) {}
pub fn fullscreen_key_pressed() -> bool {
    false
}
pub fn front_belongs_to(_processes: &[u32]) -> bool {
    false
}
