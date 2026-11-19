//! Descriptions of the original mods' configuration screens.
//! AppleSkin 3.0.10 (Unlicense) and Chunk Animator 1.0.0 (MIT).
use atlas_client::lang;
use serde_json::Value;
#[path = "config_gamma.rs"]
mod gamma;
#[path = "config_zoomify.rs"]
mod zoomify;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Frame {
    Cloth,
    Yacl,
}

#[derive(Clone, Debug)]
pub(super) struct Definition {
    pub title: String,
    pub frame: Frame,
    pub categories: Vec<Category>,
}

#[derive(Clone, Debug)]
pub(super) struct Category {
    pub label: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub(super) struct Field {
    /// An RFC 6901 pointer into the existing original mod configuration.
    pub path: String,
    pub label: String,
    pub tooltip: Vec<String>,
    pub kind: Kind,
    pub display: Option<fn(&Value) -> String>,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Kind {
    Bool,
    Int {
        min: i64,
        max: i64,
        step: i64,
        slider: bool,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
        slider: bool,
    },
    Enum {
        values: Vec<Value>,
        labels: Vec<String>,
    },
    Group {
        fields: Vec<Field>,
    },
    Color {
        alpha: bool,
    },
    Action {
        action: String,
        button: String,
    },
    Label,
}

impl Field {
    pub(super) fn new(path: &str, label: &str, kind: Kind) -> Self {
        Self {
            path: path.into(),
            label: lang::get(label),
            tooltip: Vec::new(),
            kind,
            display: None,
            enabled: true,
        }
    }

    pub(super) fn numeric(&self, text: &str) -> Option<Value> {
        match self.kind {
            Kind::Int { min, max, .. } => text
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|n| *n >= min && *n <= max)
                .map(Value::from),
            Kind::Float { min, max, .. } => text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= min && *n <= max)
                .map(Value::from),
            _ => None,
        }
    }
}

pub(super) fn definition(id: &str) -> Option<Definition> {
    if id == super::ias::ID {
        return Some(super::ias::definition());
    }
    if id == super::totem_tweaks::ID {
        return Some(super::totem_tweaks::definition());
    }
    if id == "atlas-gammautils" {
        return Some(gamma::definition());
    }
    if id == "atlas-zoomify" {
        return Some(zoomify::definition());
    }
    let (title, category, fields) = match id {
        "atlas-appleskin" => {
            let mut fields = [
                "showFoodValuesInTooltip",
                "showFoodValuesInTooltipAlways",
                "showSaturationHudOverlay",
                "showFoodValuesHudOverlay",
                "showFoodValuesHudOverlayWhenOffhand",
                "showFoodExhaustionHudUnderlay",
                "showFoodHealthHudOverlay",
                "showVanillaAnimationsOverlay",
                "maxHudOverlayFlashAlpha",
            ]
            .into_iter()
            .map(|name| {
                let key = format!("text.autoconfig.appleskin.option.{name}");
                let mut field = Field::new(
                    &format!("/{name}"),
                    &key,
                    if name == "maxHudOverlayFlashAlpha" {
                        Kind::Float {
                            min: -f64::MAX,
                            max: f64::MAX,
                            step: 0.0,
                            slider: false,
                        }
                    } else {
                        Kind::Bool
                    },
                );
                let tip_key = format!("{key}.@Tooltip");
                let tip = lang::get(&tip_key);
                if tip != tip_key {
                    field.tooltip = tip.lines().map(str::to_owned).collect();
                }
                field
            })
            .collect::<Vec<_>>();
            // AutoConfig preserves declaration order, including its trailing float.
            (
                lang::get("text.autoconfig.appleskin.title"),
                lang::get("text.autoconfig.appleskin.category.default"),
                std::mem::take(&mut fields),
            )
        }
        "atlas-chunkanimator" => {
            let enum_field = |path: &str, key: &str, names: &[&str]| {
                Field::new(
                    path,
                    key,
                    Kind::Enum {
                        values: names.iter().map(|s| Value::String((*s).into())).collect(),
                        labels: names.iter().map(|s| (*s).into()).collect(),
                    },
                )
            };
            (
                lang::get("atlas.config.chunkanimator.title"),
                lang::get("atlas.config.general"),
                vec![
                    enum_field(
                        "/mode",
                        "atlas.config.chunkanimator.mode",
                        &[
                            "BELOW",
                            "ABOVE",
                            "HYBRID",
                            "HORIZONTAL_SLIDE",
                            "HORIZONTAL_SLIDE_ALTERNATE",
                        ],
                    ),
                    enum_field(
                        "/easingFunction",
                        "atlas.config.chunkanimator.easing",
                        &[
                            "LINEAR", "QUAD", "CUBIC", "QUART", "QUINT", "EXPO", "SINE", "CIRC",
                            "BACK", "BOUNCE", "ELASTIC",
                        ],
                    ),
                    Field::new(
                        "/animationDuration",
                        "atlas.config.chunkanimator.duration",
                        Kind::Int {
                            min: 0,
                            max: 10000,
                            step: 1,
                            slider: true,
                        },
                    ),
                    Field::new(
                        "/disableAroundPlayer",
                        "atlas.config.chunkanimator.disable",
                        Kind::Bool,
                    ),
                ],
            )
        }
        _ => return None,
    };
    Some(Definition {
        title,
        frame: Frame::Cloth,
        categories: vec![Category {
            label: category,
            fields,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_editors_reject_transient_invalid_values_without_clamping_them() {
        let field = Field {
            path: "/duration".into(),
            label: String::new(),
            tooltip: vec![],
            kind: Kind::Int {
                min: 0,
                max: 10000,
                step: 1,
                slider: true,
            },
            display: None,
            enabled: true,
        };
        assert_eq!(field.numeric("1000"), Some(Value::from(1000)));
        assert_eq!(field.numeric("10000"), Some(Value::from(10000)));
        for text in ["", "-", "10001", "-1", "0.5"] {
            assert_eq!(field.numeric(text), None, "{text}");
        }
        let field = Field {
            kind: Kind::Float {
                min: -f64::MAX,
                max: f64::MAX,
                step: 0.0,
                slider: false,
            },
            ..field
        };
        assert_eq!(field.numeric("0.65"), Some(Value::from(0.65)));
        for text in ["", "NaN", "Infinity", "0,65"] {
            assert_eq!(field.numeric(text), None, "{text}");
        }
    }
}
