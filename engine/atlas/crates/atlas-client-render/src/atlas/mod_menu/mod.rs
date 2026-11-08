//! Faithful Mod Menu 21.0.0 native port. Original author and MIT licence: LICENSE-ModMenu.txt.
pub mod config;
pub mod model;
mod screen;
pub use model::Details;
pub use screen::{Action, ModMenu};

pub fn images() -> Vec<(&'static str, &'static [u8])> {
    vec![
        (
            "atlas/modmenu/icons/atlas-appleskin",
            include_bytes!("assets/atlas-appleskin.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-armorindicator",
            include_bytes!("assets/atlas-armorindicator.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-autocraft",
            include_bytes!("assets/atlas-autocraft.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-chunkanimator",
            include_bytes!("assets/atlas-chunkanimator.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-crystaloptimizer",
            include_bytes!("assets/atlas-crystaloptimizer.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-cullparticles",
            include_bytes!("assets/atlas-cullparticles.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-gammautils",
            include_bytes!("assets/atlas-gammautils.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-nofluid",
            include_bytes!("assets/atlas-nofluid.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-noshakehand",
            include_bytes!("assets/atlas-noshakehand.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-pvpsurvived",
            include_bytes!("assets/atlas-pvpsurvived.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-radarmod",
            include_bytes!("assets/atlas-radarmod.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-rwjoiner",
            include_bytes!("assets/atlas-rwjoiner.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-streamermode",
            include_bytes!("assets/atlas-streamermode.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-viewmodel",
            include_bytes!("assets/atlas-viewmodel.png"),
        ),
        (
            "atlas/modmenu/icons/atlas-zoomify",
            include_bytes!("assets/atlas-zoomify.png"),
        ),
        (
            "atlas/modmenu/mod_configuration",
            include_bytes!("assets/modmenu/gui/mod_configuration.png"),
        ),
        (
            "atlas/modmenu/parent_mod",
            include_bytes!("assets/modmenu/gui/parent_mod.png"),
        ),
        (
            "gui/sprites/modmenu/configure/disabled",
            include_bytes!("assets/modmenu/gui/sprites/configure/disabled.png"),
        ),
        (
            "gui/sprites/modmenu/configure/enabled",
            include_bytes!("assets/modmenu/gui/sprites/configure/enabled.png"),
        ),
        (
            "gui/sprites/modmenu/configure/focused",
            include_bytes!("assets/modmenu/gui/sprites/configure/focused.png"),
        ),
        (
            "gui/sprites/modmenu/filters/disabled",
            include_bytes!("assets/modmenu/gui/sprites/filters/disabled.png"),
        ),
        (
            "gui/sprites/modmenu/filters/enabled",
            include_bytes!("assets/modmenu/gui/sprites/filters/enabled.png"),
        ),
        (
            "gui/sprites/modmenu/filters/focused",
            include_bytes!("assets/modmenu/gui/sprites/filters/focused.png"),
        ),
        (
            "gui/sprites/modmenu/mods/disabled",
            include_bytes!("assets/modmenu/gui/sprites/mods/disabled.png"),
        ),
        (
            "gui/sprites/modmenu/mods/enabled",
            include_bytes!("assets/modmenu/gui/sprites/mods/enabled.png"),
        ),
        (
            "gui/sprites/modmenu/mods/focused",
            include_bytes!("assets/modmenu/gui/sprites/mods/focused.png"),
        ),
        (
            "atlas/modmenu/icon",
            include_bytes!("assets/modmenu-icon.png"),
        ),
    ]
}

static DISPLAYED_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(2);
pub fn publish_count(list: &crate::mod_list::ModList) {
    DISPLAYED_COUNT.store(
        model::count(&model::entries(list), &list.menu),
        std::sync::atomic::Ordering::Relaxed,
    );
    list.menu.publish();
}
pub fn count_text() -> String {
    let n = DISPLAYED_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let config = config::current();
    let key = if config.easter_eggs && matches!(n, 69 | 420) {
        format!("modmenu.mods.{n}.secret")
    } else if n == 1 {
        "modmenu.mods.1".into()
    } else {
        "modmenu.mods.n".into()
    };
    crate::text::Text::translate(&key, &[crate::text::Text::literal(n.to_string())]).plain()
}
pub fn button_text() -> String {
    let mut label = crate::lang::get("modmenu.title");
    if config::current().mod_count_location.button() {
        label.push_str(&count_text());
    }
    label
}
