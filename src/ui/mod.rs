pub mod group;
pub mod help;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WindowType {
    Group,
    Help,
}
