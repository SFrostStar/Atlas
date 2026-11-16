//! Gamma Utils 2.2.6 screen metadata, observed from the original running mod.
//! Original labels come from its published language resources; no implementation source is used.
use super::{Category, Definition, Field, Frame, Kind};
use atlas_client::lang;

fn field(path: &str, kind: Kind) -> Field {
    let key = format!(
        "text.autoconfig.gammautils.option.{}",
        path.trim_start_matches('/').replace('/', ".")
    );
    let mut field = Field::new(path, &key, kind);
    let tip_key = format!("{key}.@Tooltip");
    let tip = lang::get(&tip_key);
    if tip != tip_key {
        field.tooltip = tip.lines().map(str::to_owned).collect();
    }
    field
}
fn boolean(path: &str) -> Field {
    field(path, Kind::Bool)
}
fn integer(path: &str) -> Field {
    field(
        path,
        Kind::Int {
            min: i32::MIN as i64,
            max: i32::MAX as i64,
            step: 1,
            slider: false,
        },
    )
}
fn slider(path: &str, min: i64, max: i64) -> Field {
    field(
        path,
        Kind::Int {
            min,
            max,
            step: 1,
            slider: true,
        },
    )
}
fn group(path: &str, fields: Vec<Field>) -> Field {
    field(path, Kind::Group { fields })
}
fn channel(night_vision: bool) -> Category {
    let name = if night_vision { "nightVision" } else { "gamma" };
    let path = |field: &str| format!("/{name}/{field}");
    let strength = if night_vision { "NightVision" } else { "Gamma" };
    let mut fields = Vec::new();
    if !night_vision {
        fields.push(integer(&path("defaultGamma")));
    }
    fields.extend([
        integer(&path(&format!("toggled{strength}"))),
        boolean(&path("updateToggle")),
        integer(&path(if night_vision {
            "nightVisionStep"
        } else {
            "gammaStep"
        })),
        boolean(&path("showStatusEffect")),
        boolean(&path("resetOnClose")),
        group(
            &path("transition"),
            vec![
                boolean(&path("transition/smoothTransition")),
                integer(&path("transition/transitionSpeed")),
            ],
        ),
        group(
            &path("dynamic"),
            vec![
                boolean(&path("dynamic/enabled")),
                integer(&path(&format!("dynamic/min{strength}"))),
                integer(&path(&format!("dynamic/max{strength}"))),
                integer(&path("dynamic/transitionSpeed")),
                slider(&path("dynamic/averagingLightRange"), 0, 16),
                slider(&path("dynamic/skyBrightnessOverride"), 0, 100),
            ],
        ),
        group(
            &path("dimensionPreference"),
            vec![
                boolean(&path("dimensionPreference/enabled")),
                integer(&path("dimensionPreference/overworldPreference")),
                integer(&path("dimensionPreference/netherPreference")),
                integer(&path("dimensionPreference/endPreference")),
            ],
        ),
        group(
            &path("limiter"),
            vec![
                boolean(&path("limiter/limitCheck")),
                integer(&path(&format!("limiter/min{strength}"))),
                integer(&path(&format!("limiter/max{strength}"))),
            ],
        ),
    ]);
    let mut hud = vec![boolean(&path("hudMessage/showMessage"))];
    for color in ["defaultColor", "positiveColor", "negativeColor"] {
        hud.push(field(
            &path(&format!("hudMessage/{color}")),
            Kind::Color { alpha: false },
        ));
    }
    if night_vision {
        for color in ["enabledColor", "disabledColor"] {
            hud.push(field(
                &path(&format!("hudMessage/{color}")),
                Kind::Color { alpha: false },
            ));
        }
    }
    fields.push(group(&path("hudMessage"), hud));
    Category {
        label: lang::get(if night_vision {
            "text.autoconfig.gammautils.category.nightVisionSettings"
        } else {
            "text.autoconfig.gammautils.category.gammaSettings"
        }),
        fields,
    }
}
pub(super) fn definition() -> Definition {
    Definition {
        title: lang::get("text.autoconfig.gammautils.title"),
        frame: Frame::Cloth,
        categories: vec![
            channel(false),
            channel(true),
            Category {
                label: lang::get("text.autoconfig.gammautils.category.otherSettings"),
                fields: vec![boolean("/other/namespacedCommands")],
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
    fn original_categories_exclude_internal_runtime_fields_and_preserve_color_editors() {
        let d = definition();
        assert_eq!(d.categories.len(), 3);
        let all = d
            .categories
            .iter()
            .flat_map(|c| leaves(&c.fields))
            .collect::<Vec<_>>();
        assert_eq!(all.len(), 52);
        assert_eq!(
            all.iter()
                .filter(|f| matches!(f.kind, Kind::Color { alpha: false }))
                .count(),
            8
        );
        assert!(!all.iter().any(|f| f.path.ends_with("/value")
            || f.path.ends_with("/paused")
            || f.path == "/nightVision/enabled"));
        assert_eq!(d.categories[0].fields[0].path, "/gamma/defaultGamma");
        assert_eq!(
            d.categories[1].fields[0].path,
            "/nightVision/toggledNightVision"
        );
        assert_eq!(d.categories[2].fields[0].path, "/other/namespacedCommands");
    }
}
