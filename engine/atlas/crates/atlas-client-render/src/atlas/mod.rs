//! Atlas presentation and resource loading layer. Engine adapters are documented in docs/ATLAS-MODULE.md.
//! This module uses the current renderer; it is not a binary-compatible plugin.
pub mod background;
pub mod chunk_animation;
pub mod fonts;
pub mod labels;
pub mod loading;
#[doc(hidden)]
pub mod menu;
pub mod settings;

pub const NAME: &str = "Atlas";
pub const NOTICE: &str = "Not affiliated with Mojang or Microsoft";

pub fn version_label() -> String {
    format!("{NAME} 1.0.0")
}

/// Bundled artwork; individual mod assets retain the licenses in MOD-PORT-NOTICES.md.
pub fn images() -> [(&'static str, &'static [u8]); 6] {
    [
        ("atlas/loading-logo", include_bytes!("assets/logo.png")),
        ("atlas/menu-night", include_bytes!("assets/night.png")),
        (
            "atlas/appleskin-icons",
            include_bytes!("assets/appleskin-icons.png"),
        ),
        (
            "gammautils:mob_effect/bright",
            include_bytes!("assets/gammautils/bright.png"),
        ),
        (
            "gammautils:mob_effect/dim",
            include_bytes!("assets/gammautils/dim.png"),
        ),
        (
            "gammautils:mob_effect/night_vision",
            include_bytes!("assets/gammautils/night_vision.png"),
        ),
    ]
}
