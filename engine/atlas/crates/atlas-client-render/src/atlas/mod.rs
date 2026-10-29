//! Atlas presentation and resource loading layer. Engine adapters are documented in docs/ATLAS-MODULE.md.
//! This module uses the current renderer; it is not a binary-compatible plugin.
pub mod background;
pub mod fonts;
pub mod labels;
pub mod loading;
#[doc(hidden)]
pub mod menu;
pub mod settings;

pub const NAME: &str = "Atlas";
pub const NOTICE: &str = "Not affiliated with Mojang or Microsoft";

pub fn version_label() -> String {
    format!("{NAME} {}", atlas_core::VANILLA_VERSION)
}

/// Original bundled artwork, independent of downloaded Minecraft resources.
pub fn images() -> [(&'static str, &'static [u8]); 2] {
    [
        ("atlas/loading-logo", include_bytes!("assets/logo.png")),
        ("atlas/menu-night", include_bytes!("assets/night.png")),
    ]
}
