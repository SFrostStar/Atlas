//! ArmorIndicator 1.0.0 target HUD and additional damaged-equipment tint.
use super::super::{Game, ray_box};
use atlas_client::{
    hud::{HudBatch, rgba},
    items::ItemStack,
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::Instant,
};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Config {
    enabled: bool,
    visual_damage_tint: bool,
    max_overlay_alpha: f32,
    tint_red: u8,
    tint_green: u8,
    tint_blue: u8,
    target_hud_enabled: bool,
    show_durability_numbers: bool,
    show_durability_percentage: bool,
    show_durability_bar: bool,
    hud_y_offset: i32,
    target_range: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            visual_damage_tint: true,
            max_overlay_alpha: 0.55,
            tint_red: 255,
            tint_green: 20,
            tint_blue: 20,
            target_hud_enabled: true,
            show_durability_numbers: true,
            show_durability_percentage: true,
            show_durability_bar: true,
            hud_y_offset: 25,
            target_range: 24.0,
        }
    }
}
pub(super) struct State {
    config: Config,
    path: PathBuf,
    target: Cell<Option<(u32, Instant)>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            config: Config::default(),
            path: PathBuf::new(),
            target: Cell::new(None),
        }
    }
}
impl State {
    pub(super) fn load(home: &Path) -> Self {
        let path = home.join("config/armorindicator.json");
        Self {
            config: super::utility::config::load(&path),
            path,
            ..Self::default()
        }
    }
}
fn argb(n: i32) -> u32 {
    let n = n as u32;
    rgba((n >> 16) as u8, (n >> 8) as u8, n as u8, (n >> 24) as u8)
}
fn colors(f: f32) -> (u32, u32) {
    let (text, bar) = if f >= 0.7 {
        (-11141291, -13710223)
    } else if f >= 0.4 {
        (-171, -932849)
    } else if f >= 0.2 {
        (-22016, -1671646)
    } else {
        (-43691, -1618884)
    };
    (argb(text), argb(bar))
}
fn maximum(g: &Game<'_>, s: &ItemStack) -> u32 {
    if s.data()
        .and_then(|d| d.other("minecraft:unbreakable"))
        .is_some()
    {
        return 0;
    }
    s.data()
        .and_then(|d| d.other("minecraft:max_damage"))
        .and_then(|v| atlas_client::save::number(Some(v)))
        .map(|n| n.max(0) as u32)
        .unwrap_or_else(|| g.res.items().get(&s.item).map_or(0, |i| i.max_damage))
}
pub(super) fn tint(g: &Game<'_>, stack: Option<&ItemStack>) -> Option<u32> {
    let c = &g.atlas_ports.armor.config;
    if !g.atlas_port_on("atlas-armorindicator") || !c.enabled || !c.visual_damage_tint {
        return None;
    }
    let s = stack?;
    let max = maximum(g, s);
    if max == 0 || s.damage == 0 {
        return None;
    }
    let ratio = s.damage as f32 / max as f32;
    if ratio <= 0.001 {
        return None;
    }
    let alpha = (ratio * c.max_overlay_alpha * 255.0) as u32;
    Some(rgba(
        c.tint_red,
        c.tint_green,
        c.tint_blue,
        alpha.clamp(5, 255) as u8,
    ))
}
pub(super) fn draw(g: &Game<'_>, batch: &mut HudBatch) {
    let state = &g.atlas_ports.armor;
    let c = &state.config;
    if !c.enabled || !c.target_hud_enabled || g.player.is_none() {
        return;
    }
    let hit = g
        .mob_hit()
        .filter(|(i, d)| g.mobs[*i].alive() && g.before_block(*d))
        .map(|(i, _)| i)
        .or_else(|| {
            let (eye, dir) = g.aim()?;
            g.mobs
                .iter()
                .enumerate()
                .filter(|(_, m)| !m.removed && m.alive())
                .filter_map(|(i, m)| ray_box(eye, dir, c.target_range, m.bounds()).map(|d| (i, d)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        });
    if let Some(i) = hit {
        state.target.set(Some((g.mobs[i].id, Instant::now())));
    }
    let Some((id, when)) = state.target.get() else {
        return;
    };
    let Some(m) = g
        .mobs
        .iter()
        .find(|m| m.id == id && !m.removed && m.alive())
    else {
        state.target.set(None);
        return;
    };
    if hit.is_none() && when.elapsed().as_millis() > 2000 {
        state.target.set(None);
        return;
    }
    let e = &m.extra;
    let stacks: Vec<ItemStack> = [
        (e.head.as_deref(), e.armor[0].as_ref()),
        (e.chest.as_deref(), e.armor[1].as_ref()),
        (e.legs.as_deref(), e.armor[2].as_ref()),
        (e.feet.as_deref(), e.armor[3].as_ref()),
    ]
    .into_iter()
    .filter_map(|(id, s)| s.cloned().or_else(|| id.map(|id| ItemStack::new(id, 1))))
    .collect();
    if stacks.is_empty() {
        return;
    }
    let atlas = &g.gui.atlas;
    let title = g
        .remote_player_name(id)
        .map(str::to_owned)
        .or_else(|| m.custom_name.clone())
        .unwrap_or_else(|| {
            atlas_client::lang::get(&format!("entity.{}", m.kind.id.replace(':', ".")))
        });
    let name_width = HudBatch::text_width(atlas, &title);
    let rows: Vec<_> = stacks
        .iter()
        .map(|s| {
            let max = maximum(g, s);
            let rem = max.saturating_sub(s.damage);
            let f = if max == 0 {
                1.0
            } else {
                rem as f32 / max as f32
            };
            let pct = (f * 100.0 + 0.5).floor() as u32;
            let text = if max == 0 {
                g.stack_display_name(s)
                    .plain()
                    .trim_matches(['[', ']'])
                    .to_owned()
            } else {
                match (c.show_durability_numbers, c.show_durability_percentage) {
                    (true, true) => format!("{rem}/{max} ({pct}%)"),
                    (true, false) => format!("{rem}/{max}"),
                    (false, true) => format!("{pct}%"),
                    _ => String::new(),
                }
            };
            (s, max, f, text)
        })
        .collect();
    let width = rows
        .iter()
        .map(|(_, max, _, text)| {
            22.0 + if *max > 0 && c.show_durability_bar {
                55.0
            } else {
                0.0
            } + HudBatch::text_width(atlas, text)
                + 6.0
        })
        .fold(name_width + 20.0, f32::max)
        + 12.0;
    let width = width.max(140.0);
    let height = 24.0 + rows.len() as f32 * 20.0;
    let x = ((batch.width - width) / 2.0).floor();
    let y = c.hud_y_offset as f32;
    batch.fill(atlas, x, y, width, height, argb(-1341124587));
    for (dx, dy, w, h) in [
        (0.0, 0.0, width, 1.0),
        (0.0, height - 1.0, width, 1.0),
        (0.0, 0.0, 1.0, height),
        (width - 1.0, 0.0, 1.0, height),
    ] {
        batch.fill(atlas, x + dx, y + dy, w, h, argb(-2141882454));
    }
    batch.text(
        atlas,
        &title,
        x + (width - name_width) / 2.0,
        y + 6.0,
        0xffffffff,
        true,
    );
    for (row, (s, max, f, text)) in rows.iter().enumerate() {
        let py = y + 19.0 + row as f32 * 20.0;
        g.draw_item_model(batch, s, x + 6.0, py + 1.0);
        let mut tx = x + 28.0;
        let (color, bar) = colors(*f);
        if *max > 0 && c.show_durability_bar {
            batch.fill(atlas, tx, py + 7.0, 46.0, 6.0, argb(-14540250));
            batch.fill(atlas, tx, py + 7.0, (46.0 * f).floor().max(1.0), 6.0, bar);
            tx += 52.0;
        }
        batch.text(
            atlas,
            text,
            tx,
            py + 5.0,
            if *max > 0 { color } else { argb(-5592406) },
            true,
        );
    }
}
impl Game<'_> {
    pub(in crate::game) fn atlas_armor_config(&self, id: &str) -> Option<serde_json::Value> {
        (id == "atlas-armorindicator")
            .then(|| serde_json::to_value(&self.atlas_ports.armor.config).ok())
            .flatten()
    }
    pub(in crate::game) fn atlas_armor_set_config(
        &mut self,
        id: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        if id != "atlas-armorindicator" {
            return Err("Unknown mod".into());
        }
        let config: Config = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if !config.target_range.is_finite()
            || !(0.0..=256.0).contains(&config.target_range)
            || !config.max_overlay_alpha.is_finite()
            || !(0.0..=1.0).contains(&config.max_overlay_alpha)
        {
            return Err("Invalid armor range/alpha".into());
        }
        let a = &mut self.atlas_ports.armor;
        a.config = config;
        super::utility::config::save(&a.path, &a.config);
        Ok(())
    }
    pub(in crate::game) fn atlas_armor_tint(&self, s: Option<&ItemStack>) -> Option<u32> {
        tint(self, s)
    }
    pub(in crate::game) fn atlas_armor_settings(&self, id: &str, buttons: &mut Vec<String>) {
        if id != "atlas-armorindicator" {
            return;
        }
        buttons.push(atlas_client::lang::get("atlas.ports.edit_config"));
        if let Ok(serde_json::Value::Object(v)) =
            serde_json::to_value(&self.atlas_ports.armor.config)
        {
            for (k, v) in v {
                buttons.push(format!("{k}: {v}"));
            }
        }
    }
    pub(in crate::game) fn atlas_armor_settings_click(&mut self, id: &str, index: usize) {
        if id != "atlas-armorindicator" {
            return;
        }
        if index == 0 {
            self.atlas_visual_open_editor(id);
            return;
        }
        let index = index - 1;
        let a = &mut self.atlas_ports.armor;
        if let Ok(serde_json::Value::Object(mut v)) = serde_json::to_value(&a.config) {
            if let Some((_, value)) = v.iter_mut().nth(index) {
                if let Some(b) = value.as_bool() {
                    *value = serde_json::Value::Bool(!b);
                } else if let Some(n) = value.as_f64() {
                    *value = serde_json::json!(n + 1.0);
                }
            }
            if let Ok(c) = serde_json::from_value(serde_json::Value::Object(v)) {
                a.config = c;
                super::utility::config::save(&a.path, &a.config);
            }
        }
    }
}
