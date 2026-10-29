//! Typography configuration. Rasterization and font fallback belong to the engine.
pub const COMPACT: &str = "atlas:compact";
pub struct Profile {
    pub id: &'static str,
    pub bytes: &'static [u8],
    pub size: f32,
    pub oversample: f32,
}
pub fn profiles() -> [Profile; 2] {
    [
        Profile {
            id: "minecraft:default",
            bytes: atlas_theme::fonts::GEIST_REGULAR,
            size: 10.5,
            oversample: 4.0,
        },
        Profile {
            id: COMPACT,
            bytes: atlas_theme::fonts::GEIST_MEDIUM,
            size: 8.0,
            oversample: 4.0,
        },
    ]
}
