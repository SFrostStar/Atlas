//! Atlas presentation and resource loading layer. Engine adapters are documented in docs/ATLAS-MODULE.md.
//! This module uses the current renderer; it is not a binary-compatible plugin.
pub mod background;
pub mod chunk_animation;
pub mod fonts;
pub mod graphics_style;
pub mod labels;
pub mod loading;
#[doc(hidden)]
pub mod menu;
pub mod mod_menu;
pub mod mod_style;
pub mod settings;

pub const NAME: &str = "Atlas";
pub const NOTICE: &str = "Not affiliated with Mojang or Microsoft";

/// Bundled artwork; individual mod assets retain the licenses in MOD-PORT-NOTICES.md.
pub fn images() -> [(&'static str, &'static [u8]); 16] {
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
        (
            "atlas/ias/button_plain",
            include_bytes!("assets/ias/button_plain.png"),
        ),
        (
            "atlas/ias/button_focus",
            include_bytes!("assets/ias/button_focus.png"),
        ),
        (
            "atlas/ias/up_plain",
            include_bytes!("assets/ias/up_plain.png"),
        ),
        (
            "atlas/ias/up_disabled",
            include_bytes!("assets/ias/up_disabled.png"),
        ),
        (
            "atlas/ias/up_focus",
            include_bytes!("assets/ias/up_focus.png"),
        ),
        (
            "atlas/ias/down_plain",
            include_bytes!("assets/ias/down_plain.png"),
        ),
        (
            "atlas/ias/down_disabled",
            include_bytes!("assets/ias/down_disabled.png"),
        ),
        (
            "atlas/ias/down_focus",
            include_bytes!("assets/ias/down_focus.png"),
        ),
        (
            "atlas/inventoryhud/backpack",
            include_bytes!("assets/inventoryhud/backpack.png"),
        ),
        (
            "atlas/inventoryhud/ammo",
            include_bytes!("assets/inventoryhud/ammo.png"),
        ),
    ]
}
