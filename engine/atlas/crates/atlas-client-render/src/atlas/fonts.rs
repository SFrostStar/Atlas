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
            bytes: include_bytes!("../../../atlas-theme/fonts/Geist-Regular.ttf"),
            size: 10.5,
            oversample: 4.0,
        },
        Profile {
            id: COMPACT,
            bytes: include_bytes!("../../../atlas-theme/fonts/Geist-Medium.ttf"),
            size: 8.0,
            oversample: 4.0,
        },
    ]
}
