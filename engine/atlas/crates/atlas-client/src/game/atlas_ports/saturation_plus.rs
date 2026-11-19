//! Saturation Plus 1.3.1-1.21.2, MrKirbychu, CC0-1.0.
//! Adapted from the supplied SaturationOverlay, SimpleConfig and S2C payload.
use super::super::{Game, lan::LanHost};
use atlas_client::gamerules::{GameRules, SATURATION_PLUS_RULES, Value};
use atlas_client::hud::{HudAtlas, HudBatch, rgba};
use atlas_client::text::Text;
use atlas_world::nbt::Tag;
use atlas_world::saturation_plus::Snapshot;
use std::{fs, io::Write, path::Path};

pub(super) const ID: &str = "atlas-saturation-plus";
pub(in crate::game) const CHANNEL: &str = "saturation_plus:saturation";
pub(super) const CONFIG_PATH: &str = "config/saturation_plus-config.properties";
const DEFAULT_CONFIG: &str =
    "# Display saturation and exhaustion levels | Default: true\nsaturation.ui=true\n\n";
const RULES_PATH: &str = "data/saturation_plus/game_rules.dat";

pub(in crate::game) fn load_rules(world: &Path, rules: &mut GameRules) {
    rules.enable_saturation_plus();
    if let Some(saved) = atlas_client::save::read_dat(&world.join(RULES_PATH)) {
        for rule in SATURATION_PLUS_RULES {
            let value = match saved.get(rule.name) {
                Some(Tag::Byte(value)) => Value::Bool(*value != 0),
                Some(Tag::Int(value)) => Value::Int(*value),
                _ => continue,
            };
            rules.set(rule.name, value);
        }
    }
}

pub(in crate::game) fn save_rules(world: &Path, rules: &GameRules) -> atlas_client::Result<()> {
    let data = Tag::compound(SATURATION_PLUS_RULES.iter().filter_map(|rule| {
        let tag = match rules.get(rule.name)? {
            Value::Bool(value) => Tag::Byte(i8::from(value)),
            Value::Int(value) => Tag::Int(value),
        };
        Some((rule.name, tag))
    }));
    atlas_client::save::write_dat(&world.join(RULES_PATH), data)
}

pub(super) struct State {
    show_ui: bool,
    snapshot: Option<Snapshot>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            show_ui: true,
            snapshot: None,
        }
    }
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let config = home.join(CONFIG_PATH);
        if fs::metadata(&config).is_err_and(|err| err.kind() == std::io::ErrorKind::NotFound)
            && fs::create_dir_all(home.join("config")).is_ok()
            && let Ok(mut file) = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&config)
            && let Err(err) = file.write_all(DEFAULT_CONFIG.as_bytes())
        {
            log::warn!("Saturation Plus: cannot write {}: {err}", config.display());
        }
        let show_ui = fs::metadata(&config)
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() <= 16 * 1024)
            .and_then(|_| fs::read_to_string(&config).ok())
            .is_none_or(|text| parse_show_ui(&text));
        Self {
            show_ui,
            snapshot: None,
        }
    }

    pub(super) fn set_snapshot(&mut self, snapshot: Option<Snapshot>) {
        self.snapshot = snapshot;
    }

    /// This is an independent numerical HUD, drawn after the food icons.
    pub(super) fn render(&self, batch: &mut HudBatch, atlas: &HudAtlas, saturation: f32) {
        if !self.show_ui {
            return;
        }
        let Some(mut snapshot) = self.snapshot else {
            return;
        };
        snapshot.saturation = saturation;
        let Some((saturation, exhaustion)) = overlay_labels(snapshot) else {
            return;
        };
        let x = (batch.width as i32 / 2 + 92) as f32;
        let y = batch.height;
        batch.text(
            atlas,
            &saturation,
            x,
            y - 38.0,
            rgba(255, 255, 0, 255),
            true,
        );
        batch.text(
            atlas,
            &exhaustion,
            x,
            y - 28.0,
            rgba(128, 128, 128, 255),
            true,
        );
    }
}

impl Game<'_> {
    pub(in crate::game) fn saturation_plus_sync_guests(&self, host: &mut LanHost) {
        use atlas_protocol::packets::{configuration::CustomPayload, play::Clientbound};
        for guest in &mut host.hub.guests {
            let Some(snapshot) = guest
                .motion
                .as_mut()
                .and_then(|motion| motion.saturation_plus_snapshot.take())
            else {
                continue;
            };
            guest.send(&Clientbound::CustomPayload(CustomPayload {
                channel: CHANNEL.into(),
                data: encode(snapshot).to_vec(),
            }));
        }
    }

    pub(in crate::game) fn saturation_plus_reset(&mut self) {
        self.atlas_ports.saturation_plus.set_snapshot(None);
    }

    pub(in crate::game) fn saturation_plus_receive(&mut self, data: &[u8]) {
        if self.atlas_port_on(ID)
            && let Some(player) = &mut self.player
            && let Some(snapshot) = decode(data)
        {
            player.health.food.saturation = snapshot.saturation;
            player.health.food.exhaustion = snapshot.exhaustion;
            player.health.food.timer = snapshot.threshold as i32;
            // Packet updates and client-side food prediction share this cache.
            if let Some(remote) = &mut self.remote {
                remote.health.2 = snapshot.saturation;
            }
            self.atlas_ports
                .saturation_plus
                .set_snapshot(Some(snapshot));
        }
    }

    pub(in crate::game) fn saturation_plus_draw(&self, batch: &mut HudBatch) {
        if self.atlas_port_on(ID)
            && !self.creative
            && !self.spectator()
            && let Some(player) = &self.player
        {
            self.atlas_ports.saturation_plus.render(
                batch,
                &self.gui.atlas,
                player.health.food.saturation,
            );
        }
    }
}

fn parse_show_ui(text: &str) -> bool {
    let mut show = true;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            break;
        };
        if key.trim() == "saturation.ui" {
            let value = value.split_once(" #").map_or(value, |(value, _)| value);
            show = value.trim().eq_ignore_ascii_case("true");
        }
    }
    show
}

pub(super) fn encode(snapshot: Snapshot) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..4].copy_from_slice(&snapshot.saturation.to_be_bytes());
    bytes[4..8].copy_from_slice(&snapshot.exhaustion.to_be_bytes());
    bytes[8..].copy_from_slice(&snapshot.threshold.to_be_bytes());
    bytes
}

fn decode(bytes: &[u8]) -> Option<Snapshot> {
    let bytes: [u8; 12] = bytes.try_into().ok()?;
    let saturation = f32::from_be_bytes(bytes[..4].try_into().ok()?);
    let exhaustion = f32::from_be_bytes(bytes[4..8].try_into().ok()?);
    let threshold = f32::from_be_bytes(bytes[8..].try_into().ok()?);
    if [saturation, exhaustion, threshold]
        .iter()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    Some(Snapshot {
        saturation,
        exhaustion,
        threshold: threshold as i32 as f32,
    })
}

fn decimal(value: f32) -> String {
    format!("{:.2}", f64::from(value)).replace('.', atlas_threads::locale::decimal_separator())
}

fn overlay_labels(snapshot: Snapshot) -> Option<(String, String)> {
    let ratio = snapshot.exhaustion / snapshot.threshold;
    if snapshot.threshold == 0.0 || ratio.is_nan() {
        return None;
    }
    Some((
        decimal(snapshot.saturation),
        format!("{:.0}%", f64::from(ratio) * 100.0),
    ))
}

pub(in crate::game) fn saturation_text(value: f32) -> Text {
    let bars = (value / 2.0).floor();
    let bars_key = if bars == 1.0 {
        "message.hungerbar.singular"
    } else {
        "message.hungerbar.plural"
    };
    Text::translate(
        "message.saturation",
        &[
            Text::literal(decimal(value)),
            Text::literal(format!("{bars:.0}")),
            Text::translate(bars_key, &[]),
        ],
    )
}

pub(in crate::game) fn change_text(before: f32, after: f32) -> Text {
    let delta = after - before;
    if delta == 0.0 {
        return Text::translate("message.saturation.unchanged", &[]);
    }
    let magnitude = delta.abs();
    let number = if (0.001..10_000_000.0).contains(&magnitude) {
        format!("{delta:+?}")
    } else {
        let text = format!("{delta:+E}");
        if let Some((mantissa, exponent)) = text.split_once('E')
            && !mantissa.contains('.')
        {
            format!("{mantissa}.0E{exponent}")
        } else {
            text
        }
    };
    Text::tr("message.saturation.changed", &[&number])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_and_hud_keep_values_above_vanilla_caps() {
        let snapshot = Snapshot {
            saturation: 27.5,
            exhaustion: 2.0,
            threshold: 8.0,
        };
        let bytes = [0x41, 0xdc, 0, 0, 0x40, 0, 0, 0, 0x41, 0, 0, 0];
        assert_eq!(encode(snapshot), bytes);
        assert_eq!(decode(&bytes), Some(snapshot));
        let (saturation, exhaustion) = overlay_labels(snapshot).unwrap();
        assert_eq!(
            saturation,
            format!("27{}50", atlas_threads::locale::decimal_separator())
        );
        assert_eq!(exhaustion, "25%");
        assert!(decode(&bytes[..11]).is_none());
        assert!(
            decode(&encode(Snapshot {
                saturation: f32::NAN,
                ..snapshot
            }))
            .is_none()
        );
        assert!(
            overlay_labels(Snapshot {
                threshold: 0.0,
                ..snapshot
            })
            .is_none()
        );
    }

    #[test]
    fn original_properties_are_read_without_a_new_menu() {
        assert!(parse_show_ui(DEFAULT_CONFIG));
        assert!(parse_show_ui("other.setting=false\n"));
        assert!(!parse_show_ui(
            "# user choice\nsaturation.ui = FALSE # reason\n"
        ));
        assert!(parse_show_ui("saturation.ui=false\nsaturation.ui=TRUE\n"));
        assert!(!parse_show_ui(
            "saturation.ui=false\nbad line\nsaturation.ui=true\n"
        ));
    }
}
