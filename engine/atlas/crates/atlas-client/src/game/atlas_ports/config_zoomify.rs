//! Zoomify 2.15.2 configuration observed through the original public YACL API.
//! Labels and descriptions retain the original mod's translations.
use super::{Category, Definition, Field, Frame, Kind};
use atlas_client::lang;
use serde_json::Value;

fn option(path: &str, key: &str, kind: Kind) -> Field {
    let mut field = Field::new(path, key, kind);
    for i in 1..=16 {
        let description_key = format!("{key}.description.{i}");
        let text = lang::get(&description_key);
        if text == description_key {
            break;
        }
        field.tooltip.extend(text.lines().map(str::to_owned));
    }
    field
}
fn boolean(name: &str, key: &str) -> Field {
    option(&format!("/{name}"), key, Kind::Bool)
}
fn integer(
    name: &str,
    key: &str,
    min: i64,
    max: i64,
    step: i64,
    display: Option<fn(&Value) -> String>,
) -> Field {
    let mut field = option(
        &format!("/{name}"),
        key,
        Kind::Int {
            min,
            max,
            step,
            slider: true,
        },
    );
    field.display = display;
    field
}
fn seconds(name: &str, key: &str, min: f64, max: f64, step: f64) -> Field {
    let mut field = option(
        &format!("/{name}"),
        key,
        Kind::Float {
            min,
            max,
            step,
            slider: true,
        },
    );
    field.display = Some(time);
    field
}
fn enumeration(name: &str, key: &str, prefix: &str, values: &[&str]) -> Field {
    option(
        &format!("/{name}"),
        key,
        Kind::Enum {
            values: values
                .iter()
                .map(|value| Value::String(value.to_ascii_lowercase()))
                .collect(),
            labels: values
                .iter()
                .map(|value| lang::get(&format!("{prefix}.{}", value.to_ascii_lowercase())))
                .collect(),
        },
    )
}
fn group(category: &str, name: &str, fields: Vec<Field>) -> Field {
    option(
        &format!("/@groups/{category}/{name}"),
        &format!("yacl3.config.zoomify.category.{category}.group.{name}"),
        Kind::Group { fields },
    )
}
fn action(path: &str, key: &str, action: &str, button: &str) -> Field {
    option(
        path,
        key,
        Kind::Action {
            action: action.into(),
            button: lang::get(button),
        },
    )
}
fn decimal(value: f64) -> String {
    let text = format!("{:.1}", (value * 10.0).round() / 10.0);
    // Java's formatter uses the OS locale, independently of Minecraft's language.
    text.replace('.', atlas_threads::locale::decimal_separator())
}
fn time(value: &Value) -> String {
    let value = value.as_f64().unwrap_or(0.0);
    if value == 0.0 {
        lang::get("zoomify.gui.formatter.instant")
    } else {
        lang::get("zoomify.gui.formatter.seconds").replace("%s", &decimal(value))
    }
}
fn factor(value: &Value) -> String {
    format!("{}x", value.as_i64().unwrap_or(0))
}
fn factor_percent(value: &Value) -> String {
    format!("{}x", decimal(value.as_f64().unwrap_or(0.0) / 100.0))
}
fn percent(value: &Value) -> String {
    format!("{}%", value.as_i64().unwrap_or(0))
}
fn cinematic(value: &Value) -> String {
    if value.as_i64() == Some(0) {
        lang::get("options.off")
    } else {
        percent(value)
    }
}
fn transitions(name: &str, key: &str) -> Field {
    enumeration(
        name,
        key,
        "zoomify.transition",
        &[
            "INSTANT",
            "LINEAR",
            "EASE_IN_SINE",
            "EASE_OUT_SINE",
            "EASE_IN_OUT_SINE",
            "EASE_IN_QUAD",
            "EASE_OUT_QUAD",
            "EASE_IN_OUT_QUAD",
            "EASE_IN_CUBIC",
            "EASE_OUT_CUBIC",
            "EASE_IN_OUT_CUBIC",
            "EASE_IN_EXP",
            "EASE_OUT_EXP",
            "EASE_IN_OUT_EXP",
        ],
    )
}
fn preset(name: &str, translation: &str) -> Field {
    let mut field = action(
        &format!("/@actions/preset/{name}"),
        &format!("zoomify.gui.preset.{translation}"),
        &format!("zoomify.preset.{name}"),
        "yacl3.config.zoomify.category.misc.group.presets.presetBtn.button",
    );
    field.tooltip = vec![
        lang::get("yacl3.config.zoomify.category.misc.group.presets.presetBtn.description")
            .replace("%s", &field.label),
    ];
    field
}
pub(super) fn definition() -> Definition {
    let basic = "yacl3.config.zoomify.category.behaviour.group.basic.option";
    let scrolling = "yacl3.config.zoomify.category.behaviour.group.scrolling.option";
    let spyglass = "yacl3.config.zoomify.category.behaviour.group.spyglass.option";
    let controls = "yacl3.config.zoomify.category.controls.root.option";
    let secondary = "yacl3.config.zoomify.category.secondary.root.option";
    let misc = "yacl3.config.zoomify.category.misc.root.option";
    let mut migration = action(
        "/@actions/check_migrations",
        &format!("{misc}.checkMigrations"),
        "zoomify.check_migrations",
        "yacl.control.action.execute",
    );
    migration.enabled = false;
    migration.tooltip = vec![lang::get("atlas.config.zoomify.migration_unavailable")];
    Definition {
        title: lang::get("yacl3.config.zoomify.title"),
        frame: Frame::Yacl,
        categories: vec![
            Category {
                label: lang::get("yacl3.config.zoomify.category.behaviour"),
                fields: vec![
                    group(
                        "behaviour",
                        "basic",
                        vec![
                            integer(
                                "initialZoom",
                                &format!("{basic}.initialZoom"),
                                1,
                                10,
                                1,
                                Some(factor),
                            ),
                            seconds("zoomInTime", &format!("{basic}.zoomInTime"), 0.1, 5.0, 0.1),
                            seconds(
                                "zoomOutTime",
                                &format!("{basic}.zoomOutTime"),
                                0.1,
                                5.0,
                                0.1,
                            ),
                            transitions("zoomInTransition", &format!("{basic}.zoomInTransition")),
                            transitions("zoomOutTransition", &format!("{basic}.zoomOutTransition")),
                            boolean("affectHandFov", &format!("{basic}.affectHandFov")),
                        ],
                    ),
                    group(
                        "behaviour",
                        "scrolling",
                        vec![
                            boolean("scrollZoom", &format!("{scrolling}.scrollZoom")),
                            integer(
                                "scrollStepCount",
                                &format!("{scrolling}.scrollStepCount"),
                                3,
                                20,
                                1,
                                None,
                            ),
                            integer(
                                "zoomPerStep",
                                &format!("{scrolling}.zoomPerStep"),
                                110,
                                200,
                                10,
                                Some(factor_percent),
                            ),
                            integer(
                                "scrollZoomSmoothness",
                                &format!("{scrolling}.scrollZoomSmoothness"),
                                0,
                                100,
                                1,
                                Some(percent),
                            ),
                            boolean("retainZoomSteps", &format!("{scrolling}.retainZoomSteps")),
                        ],
                    ),
                    group(
                        "behaviour",
                        "spyglass",
                        vec![
                            enumeration(
                                "spyglassBehaviour",
                                &format!("{spyglass}.spyglassBehaviour"),
                                "zoomify.spyglass_behaviour",
                                &[
                                    "COMBINE",
                                    "OVERRIDE",
                                    "ONLY_ZOOM_WHILE_HOLDING",
                                    "ONLY_ZOOM_WHILE_CARRYING",
                                ],
                            ),
                            enumeration(
                                "spyglassOverlayVisibility",
                                &format!("{spyglass}.spyglassOverlayVisibility"),
                                "zoomify.overlay_visibility",
                                &["NEVER", "HOLDING", "CARRYING", "ALWAYS"],
                            ),
                            enumeration(
                                "spyglassSoundBehaviour",
                                &format!("{spyglass}.spyglassSoundBehaviour"),
                                "zoomify.sound_behaviour",
                                &["NEVER", "ALWAYS", "ONLY_SPYGLASS", "WITH_OVERLAY"],
                            ),
                        ],
                    ),
                ],
            },
            Category {
                label: lang::get("yacl3.config.zoomify.category.controls"),
                fields: vec![
                    enumeration(
                        "zoomKeyBehaviour",
                        &format!("{controls}.zoomKeyBehaviour"),
                        "zoomify.zoom_key_behaviour",
                        &["HOLD", "TOGGLE"],
                    ),
                    boolean(
                        "_keybindScrolling",
                        &format!("{controls}._keybindScrolling"),
                    ),
                    integer(
                        "relativeSensitivity",
                        &format!("{controls}.relativeSensitivity"),
                        0,
                        150,
                        10,
                        Some(percent),
                    ),
                    boolean(
                        "relativeViewBobbing",
                        &format!("{controls}.relativeViewBobbing"),
                    ),
                    integer(
                        "cinematicCamera",
                        &format!("{controls}.cinematicCamera"),
                        0,
                        250,
                        10,
                        Some(cinematic),
                    ),
                ],
            },
            Category {
                label: lang::get("yacl3.config.zoomify.category.secondary"),
                fields: vec![
                    Field::new(
                        "/@labels/secondary",
                        "yacl3.config.zoomify.category.secondary.root.label.infoLabel",
                        Kind::Label,
                    ),
                    integer(
                        "secondaryZoomAmount",
                        &format!("{secondary}.secondaryZoomAmount"),
                        2,
                        10,
                        1,
                        Some(factor),
                    ),
                    seconds(
                        "secondaryZoomInTime",
                        &format!("{secondary}.secondaryZoomInTime"),
                        6.0,
                        30.0,
                        2.0,
                    ),
                    seconds(
                        "secondaryZoomOutTime",
                        &format!("{secondary}.secondaryZoomOutTime"),
                        0.0,
                        5.0,
                        0.25,
                    ),
                    boolean(
                        "secondaryHideHUDOnZoom",
                        &format!("{secondary}.secondaryHideHUDOnZoom"),
                    ),
                ],
            },
            Category {
                label: lang::get("yacl3.config.zoomify.category.misc"),
                fields: vec![
                    action(
                        "/@actions/unbind_conflicting",
                        &format!("{misc}.unbindConflicting"),
                        "zoomify.unbind_conflicting",
                        "yacl.control.action.execute",
                    ),
                    migration,
                    group(
                        "misc",
                        "presets",
                        vec![
                            Field::new(
                                "/@labels/presets",
                                "yacl3.config.zoomify.category.misc.group.presets.label.applyWarning",
                                Kind::Label,
                            ),
                            preset("default", "default"),
                            preset("optifine", "optifine"),
                            preset("okzoomer", "ok_zoomer"),
                        ],
                    ),
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn leaves(fields: &[Field]) -> Vec<&Field> {
        fields
            .iter()
            .flat_map(|field| {
                if let Kind::Group { fields } = &field.kind {
                    leaves(fields)
                } else {
                    vec![field]
                }
            })
            .collect()
    }
    #[test]
    fn original_twenty_three_config_fields_and_disabled_migration() {
        let d = definition();
        assert_eq!(d.frame, Frame::Yacl);
        assert_eq!(d.categories.len(), 4);
        let all = d
            .categories
            .iter()
            .flat_map(|category| leaves(&category.fields))
            .collect::<Vec<_>>();
        let config = all
            .iter()
            .filter(|field| !matches!(field.kind, Kind::Action { .. } | Kind::Label))
            .collect::<Vec<_>>();
        assert_eq!(config.len(), 23);
        assert!(
            config
                .iter()
                .any(|field| field.path == "/_keybindScrolling")
        );
        assert!(
            config
                .iter()
                .any(|field| field.path == "/secondaryHideHUDOnZoom")
        );
        for field in &config {
            if let Kind::Enum { values, labels } = &field.kind {
                assert_eq!(values.len(), labels.len());
                assert!(
                    values
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| s == s.to_ascii_lowercase()))
                );
            }
        }
        assert_eq!(
            all.iter()
                .filter(|field| matches!(field.kind, Kind::Action { .. }))
                .count(),
            5
        );
        let migration=all.iter().find(|field|matches!(&field.kind,Kind::Action{action,..}if action=="zoomify.check_migrations")).unwrap();
        assert!(!migration.enabled);
        assert_eq!(
            migration.tooltip,
            vec![lang::get("atlas.config.zoomify.migration_unavailable")]
        );
    }
}
