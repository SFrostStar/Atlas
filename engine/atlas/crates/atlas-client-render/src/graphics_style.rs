//! Stable engine adapter for Atlas's settings presentation.
//! Keep Atlas's visual implementation in its module across upstream updates.
pub use crate::atlas::graphics_style::*;

pub fn accent() -> u32 { ACCENT }
pub fn head_rule() -> u32 { HEAD_RULE }
pub fn hover() -> u32 { HOVER }
pub fn muted() -> u32 { MUTED }
pub fn open() -> u32 { OPEN }
pub fn row_rule() -> u32 { ROW_RULE }
pub fn soft() -> u32 { SOFT }
pub fn veil() -> u32 { VEIL }
