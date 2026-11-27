//! Atlas's configurable minimap and local markers. Terrain sampling stays in the
//! budgeted minimap renderer shared with native mods.
use super::{
    Game,
    minimap_settings::{BINDINGS, CaveMode, ID},
    minimap_waypoints::label as waypoint_label,
};
use crate::game::{Screen, native_hooks::NativeMinimap};
use atlas_client::{
    extras::Corner,
    extras_hud::Corners,
    hud::{HudAtlas, HudBatch, Rect, rgba},
    lang,
    mesher::Vertex,
    minimap::{self, Layer, View},
    mobs::{Mob, Part, anim::Poses, render},
    ui_theme::{Face, Ui, color},
};
use glam::{DVec3, Mat4};
use winit::keyboard::KeyCode;

const MAX_RADAR_MARKERS: usize = 256;
const MAX_WORLD_LABELS: usize = 32;
const MOB_DOT: u32 = rgba(255, 226, 81, 255);
const ITEM_DOT: u32 = rgba(244, 61, 55, 255);
const MARKER_SIZE: f32 = 8.0;

#[derive(Default)]
pub(super) struct Runtime {
    world: Option<String>,
    enlarged: bool,
    hidden_waypoints: bool,
    dead: bool,
}

fn label(name: &str) -> String {
    lang::get(&format!("atlas.minimap.{name}"))
}

impl Game<'_> {
    pub(in crate::game) fn atlas_minimap_world_key(&self) -> Option<String> {
        if self.title.is_some() || self.player.is_none() {
            return None;
        }
        if let Some(remote) = &self.remote {
            // The integrated server owns world_root; its attached client gets the save folder.
            if let Some(folder) = &remote.save_folder {
                return Some(format!("world:{folder}|{}", remote.level));
            }
            if self.integrated.is_none() {
                let address = self.server_address()?.trim().to_ascii_lowercase();
                return Some(format!(
                    "server:{address}|{}|{}",
                    remote.level, remote.hashed_seed
                ));
            }
        }
        let world = self.world_root.file_name()?.to_string_lossy();
        Some(format!("world:{world}|{}", self.dim.id()))
    }

    pub(in crate::game) fn atlas_minimap_frame(&mut self) {
        let key = self
            .atlas_port_on(ID)
            .then(|| self.atlas_minimap_world_key())
            .flatten();
        let state = &mut self.atlas_ports.minimap_runtime;
        if key != state.world {
            self.atlas_ports.minimap_waypoints.clear_temporary();
            *state = Runtime {
                world: key,
                ..Runtime::default()
            };
        }
        if state.world.is_none() {
            return;
        }
        let options = self.atlas_ports.minimap;
        if !options.enlarge_toggle
            && !(self.screen == Screen::Playing && (self.focused || self.control.is_some()))
        {
            state.enlarged = false;
        }
        let dead = self.screen == Screen::Dead;
        let new_death = dead && !state.dead;
        state.dead = dead;
        if new_death
            && options.death_waypoints
            && let Some(p) = &self.player
        {
            let pos = p.position;
            self.atlas_minimap_death_waypoint([
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            ]);
        }
    }

    pub(in crate::game) fn atlas_minimap_request(&self) -> Option<NativeMinimap> {
        if !self.atlas_port_on(ID) {
            return None;
        }
        self.atlas_ports
            .minimap
            .request(self.atlas_ports.minimap_runtime.enlarged)
    }

    pub(in crate::game) fn atlas_minimap_layer(&self, head: i32, automatic: Layer) -> Layer {
        if self.atlas_minimap_request().is_none() {
            return automatic;
        }
        match self.atlas_ports.minimap.cave {
            CaveMode::Auto => automatic,
            CaveMode::Surface => Layer::Surface,
            CaveMode::Cave => Layer::Below(head),
        }
    }

    pub(in crate::game) fn atlas_minimap_grid(&self) -> bool {
        self.atlas_ports.minimap.grid && self.atlas_minimap_request().is_some()
    }

    pub(in crate::game) fn atlas_minimap_portraits(&self) -> bool {
        self.atlas_ports.minimap.radar
            && self.screen == Screen::Playing
            && self.physical_keys.contains(&KeyCode::Tab)
    }

    pub(in crate::game) fn atlas_minimap_input(&mut self, key: &str, pressed: bool) {
        if !self.atlas_port_on(ID) {
            return;
        }
        let Some(&(action, _)) = BINDINGS
            .iter()
            .find(|&&(action, default)| self.native_mod_key(ID, action, default) == key)
        else {
            return;
        };
        let mut options = self.atlas_ports.minimap;
        if action == "enlarge" && !options.enlarge_toggle && !pressed {
            self.atlas_ports.minimap_runtime.enlarged = false;
        }
        if !pressed || !self.native_playing() || self.player.is_none() {
            return;
        }
        match action {
            "settings" => {
                self.atlas_config_open(ID);
                return;
            }
            "new_waypoint" => {
                self.atlas_minimap_waypoints_open(true);
                return;
            }
            "waypoints" => {
                self.atlas_minimap_waypoints_open(false);
                return;
            }
            "temporary_waypoint" => {
                self.atlas_minimap_temporary_waypoint_open();
                return;
            }
            "enlarge" => {
                let enlarged = &mut self.atlas_ports.minimap_runtime.enlarged;
                *enlarged = !options.enlarge_toggle || !*enlarged;
                return;
            }
            "toggle_waypoints" => {
                let hidden = &mut self.atlas_ports.minimap_runtime.hidden_waypoints;
                *hidden = !*hidden;
                return;
            }
            "toggle_map" => options.enabled = !options.enabled,
            "toggle_map_waypoints" => options.map_waypoints = !options.map_waypoints,
            "toggle_radar" => options.radar = !options.radar,
            "toggle_grid" => options.grid = !options.grid,
            "toggle_cave" => {
                options.cave = if matches!(options.cave, CaveMode::Surface) {
                    CaveMode::Auto
                } else {
                    CaveMode::Surface
                };
            }
            "zoom_in" => {
                options.change_zoom(1);
            }
            "zoom_out" => {
                options.change_zoom(-1);
            }
            _ => return,
        }
        if options.save(&Self::game_dir()).is_ok() {
            self.atlas_ports.minimap = options;
        }
    }

    pub(in crate::game) fn draw_atlas_minimap(
        &self,
        batch: &mut HudBatch,
        corners: &mut Corners,
        corner: Corner,
        image: Rect,
        view: View,
        paint: bool,
    ) {
        let Some(player) = &self.player else {
            return;
        };
        let options = self.atlas_ports.minimap;
        let state = &self.atlas_ports.minimap_runtime;
        let size = options.size_for_view([batch.width, batch.height], state.enlarged);
        let p = player.position;
        let xyz = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
        let mut lines = Vec::with_capacity(4);
        if options.coordinates {
            lines.push(format!("{}, {}, {}", xyz[0], xyz[1], xyz[2]));
        }
        if options.biome
            && let Some(biome) = self.biome_name_at_block(xyz)
        {
            lines.push(lang::get(&format!("biome.{}", biome.replace(':', "."))));
        }
        if options.light {
            let light = self.world.light(xyz[0], xyz[1], xyz[2]);
            lines.push(format!(
                "{} {}",
                label("light_short"),
                (light & 15).max(light >> 4)
            ));
        }
        if options.time {
            let minutes = (self.view_day_time() + 6000).rem_euclid(24000) * 1440 / 24000;
            lines.push(format!("{:02}:{:02}", minutes / 60, minutes % 60));
        }
        let line_height = 11.0;
        let h = size + lines.len() as f32 * line_height;
        let [x, y] = if state.enlarged {
            [(batch.width - size) / 2.0, (batch.height - h) / 2.0]
        } else {
            let (x, y) = corners.place(corner, size, h);
            [x, y]
        };
        if !paint {
            return;
        }
        let half = size * 0.5;
        let center = [x + half, y + half];
        let atlas = &self.gui.atlas;
        batch.rect_uv(x, y, size, size, image, u32::MAX);
        let from_texel = size / minimap::SIZE as f32;
        let frame = 3.0 * from_texel;
        let bounds = |x: f64, z: f64, inset: f32, edge: bool| {
            let offset = view.project(x, z).map(|component| component * from_texel);
            let radius = (half - frame - inset).max(0.0);
            let reach = if view.round {
                offset[0].hypot(offset[1])
            } else {
                offset[0].abs().max(offset[1].abs())
            };
            if !reach.is_finite() || (reach > radius && !edge) {
                return None;
            }
            let scale = if reach > radius { radius / reach } else { 1.0 };
            Some([center[0] + scale * offset[0], center[1] + scale * offset[1]])
        };

        let mut names = Vec::new();
        if options.radar {
            let portraits = self.atlas_minimap_portraits();
            let height_ok = |delta: f64| {
                options.radar_height == 0 || delta.abs() <= f64::from(options.radar_height)
            };
            let mut mob_count = 0;
            for mob in self.mobs.iter() {
                let is_player = mob.kind.id == "minecraft:player";
                if (if is_player {
                    !options.radar_players
                } else {
                    !options.radar_mobs || mob_count == MAX_RADAR_MARKERS
                }) || mob.extra.spectator
                    || mob.removed
                    || mob.health <= 0.0
                    || mob.is_invisible()
                    || !height_ok(mob.position.y - p.y)
                {
                    continue;
                }
                let inset = if is_player { 10.0 } else { 4.5 };
                let Some(point) = bounds(mob.position.x, mob.position.z, inset, false) else {
                    continue;
                };
                if is_player {
                    draw_radar_dot(batch, atlas, point, u32::MAX, 1.5);
                    if let Some(name) = self.remote_player_name(mob.id) {
                        names.push((point, name));
                    }
                } else {
                    if !portraits || !draw_mob_head(batch, atlas, point, mob) {
                        draw_radar_dot(batch, atlas, point, MOB_DOT, 1.25);
                    }
                    mob_count += 1;
                }
            }
            if options.radar_items {
                for (point, stack) in self
                    .dropped
                    .iter()
                    .filter(|item| height_ok(item.position.y - p.y))
                    .filter_map(|item| {
                        Some((
                            bounds(item.position.x, item.position.z, 4.5, false)?,
                            &item.stack,
                        ))
                    })
                    .take(MAX_RADAR_MARKERS)
                {
                    let start = batch.vertices.len();
                    if portraits {
                        // Draw at a fixed origin so moving items share the inventory's model cache.
                        self.draw_item_model(batch, stack, 0.0, 0.0);
                    }
                    if start == batch.vertices.len() {
                        draw_radar_dot(batch, atlas, point, ITEM_DOT, 1.25);
                    } else {
                        let [px, py] = point.map(f32::round);
                        let scale = MARKER_SIZE / 16.0;
                        let half = MARKER_SIZE / 2.0;
                        batch.scale_since(start, 0.0, 0.0, scale, scale);
                        batch.translate_since(start, px - half, py - half);
                        batch.clip_since(start, px - half, py - half, px + half, py + half);
                    }
                }
            }
        }

        if options.map_waypoints
            && let Some(key) = &state.world
        {
            for waypoint in self
                .atlas_ports
                .minimap_waypoints
                .entries(key)
                .iter()
                .filter(|w| w.enabled)
            {
                let [wx, _, wz] = waypoint.position.map(f64::from);
                let distance = (wx - p.x).hypot(wz - p.z);
                if options.waypoint_max_distance != 0
                    && distance > f64::from(options.waypoint_max_distance)
                {
                    continue;
                }
                let Some(point) = bounds(wx + 0.5, wz + 0.5, 4.8, waypoint.global) else {
                    continue;
                };
                waypoint.draw_badge(batch, atlas, point, 8.0);
            }
        }
        let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
        if options.cardinal {
            let (nx, ny) = view.north();
            let reach = (half - frame - 3.0).max(0.0);
            for ((dx, dy), direction) in [
                ((nx, ny), "north"),
                ((-ny, nx), "east"),
                ((-nx, -ny), "south"),
                ((ny, -nx), "west"),
            ] {
                let text = label(direction);
                let sx = center[0] + reach * dx - ui.measure(Face::SemiBold, 8.0, &text) / 2.0;
                let sy = center[1] + reach * dy - 5.0;
                ui.text_line(
                    Face::SemiBold,
                    8.0,
                    10.0,
                    sx + 0.5,
                    sy + 0.5,
                    0xff11151b,
                    &text,
                );
                ui.text_line(Face::SemiBold, 8.0, 10.0, sx, sy, color::text(), &text);
            }
        }
        for ([px, py], name) in names {
            let left = x + frame + 1.0;
            let right = x + size - frame - 1.0;
            let max_width = right - left;
            let font = 6.5;
            let advance = ui.measure(Face::SemiBold, font, name);
            let font = font * (max_width / advance.max(1.0)).min(1.0);
            let width = ui.measure(Face::SemiBold, font, name);
            let sx = (px - width / 2.0).clamp(left, (right - width).max(left));
            ui.text_line(
                Face::SemiBold,
                font,
                8.0,
                sx + 0.5,
                py + 2.5,
                0xff11151b,
                name,
            );
            ui.text_line(Face::SemiBold, font, 8.0, sx, py + 2.0, u32::MAX, name);
        }
        for (i, text) in lines.iter().enumerate() {
            let w = ui.measure(Face::Medium, 8.0, text);
            let sx = center[0] - w / 2.0;
            let sy = y + size + i as f32 * line_height;
            ui.rounded(
                sx - 3.0,
                sy + 0.5,
                w + 6.0,
                line_height - 1.0,
                3.0,
                0xd8181e26,
            );
            ui.text_line(Face::Medium, 8.0, line_height, sx, sy, color::text(), text);
        }
        ui.finish();

        let local = bounds(p.x, p.z, 1.0, false).unwrap_or(center);
        draw_radar_dot(batch, atlas, local, u32::MAX, 1.5);
    }

    pub(in crate::game) fn atlas_minimap_world_labels_shown(&self) -> bool {
        let state = &self.atlas_ports.minimap_runtime;
        !self.ui.hide_gui
            && self.native_window()
            && self.atlas_port_on(ID)
            && self.atlas_ports.minimap.world_waypoints
            && !state.hidden_waypoints
            && state.world.as_ref().is_some_and(|key| {
                self.atlas_ports
                    .minimap_waypoints
                    .entries(key)
                    .iter()
                    .any(|point| point.enabled)
            })
    }

    pub(in crate::game) fn draw_atlas_minimap_world_labels(
        &self,
        batch: &mut HudBatch,
        camera: DVec3,
        view_projection: Mat4,
    ) {
        let options = self.atlas_ports.minimap;
        let Some(key) = &self.atlas_ports.minimap_runtime.world else {
            return;
        };
        let visible: Vec<_> = self
            .atlas_ports
            .minimap_waypoints
            .entries(key)
            .iter()
            .rev()
            .filter(|point| point.enabled)
            .filter_map(|point| {
                let offset =
                    DVec3::from_array(point.position.map(f64::from)) + DVec3::splat(0.5) - camera;
                let distance = offset.length();
                if distance <= 1.0
                    || (options.waypoint_max_distance != 0
                        && distance > f64::from(options.waypoint_max_distance))
                {
                    return None;
                }
                // Only project onto the screen. World depth and fog never affect these labels.
                let clip = view_projection * offset.as_vec3().extend(1.0);
                if clip.w <= 0.0 || clip.x.abs() > clip.w || clip.y.abs() > clip.w {
                    return None;
                }
                let x = (clip.x / clip.w + 1.0) * batch.width * 0.5;
                let y = (1.0 - clip.y / clip.w) * batch.height * 0.5;
                Some((point, [x, y], distance))
            })
            .take(MAX_WORLD_LABELS)
            .collect();
        let metres = options.waypoint_distance.then(|| waypoint_label("metres"));
        let atlas = &self.gui.atlas;
        for (waypoint, [x, y], distance) in visible.into_iter().rev() {
            waypoint.draw_badge(batch, atlas, [x, y - 10.0], 15.0);
            let mut title = waypoint.display_name();
            if let Some(unit) = &metres {
                title.to_mut().push_str(&format!("  {distance:.0} {unit}"));
            }
            let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
            let font = 8.5;
            let width = ui.measure(Face::SemiBold, font, &title);
            let left = x - width / 2.0;
            ui.rounded(
                left - 4.0,
                y + 2.0,
                width + 8.0,
                13.0,
                3.0,
                rgba(18, 22, 27, 225),
            );
            ui.text_line(Face::SemiBold, font, 13.0, left, y + 2.0, u32::MAX, &title);
            ui.finish();
        }
    }
}

fn draw_radar_dot(batch: &mut HudBatch, atlas: &HudAtlas, at: [f32; 2], color: u32, radius: f32) {
    // The normal GUI disc uses too few segments at this radius.
    const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
    const RIM: [[f32; 2]; 9] = [
        [0.0, -1.0],
        [D, -D],
        [1.0, 0.0],
        [D, D],
        [0.0, 1.0],
        [-D, D],
        [-1.0, 0.0],
        [-D, -D],
        [0.0, -1.0],
    ];
    for (r, tint) in [(radius + 0.5, rgba(16, 22, 28, 210)), (radius, color)] {
        let rim = RIM.map(|[x, y]| [at[0] + x * r, at[1] + y * r]);
        batch.fill_fan(atlas, at, &rim, tint);
    }
}

fn find_part<'a>(part: &'a Part, name: &str) -> Option<&'a Part> {
    if part.name == name {
        Some(part)
    } else {
        part.children
            .iter()
            .find_map(|child| find_part(child, name))
    }
}

fn draw_mob_head(batch: &mut HudBatch, atlas: &HudAtlas, at: [f32; 2], mob: &Mob) -> bool {
    let (key, texture) = render::body(mob);
    let Some(sprite) = atlas.sprites.get(texture.as_ref()).copied() else {
        return false;
    };
    let model = mob.kind.layer(&key).unwrap_or(&mob.kind.model);
    let head = ["head_parts", "head", "center_head", "body"]
        .into_iter()
        .find_map(|name| find_part(model, name))
        .unwrap_or(model);
    let mut vertices = Vec::new();
    let tint = render::body_color(mob, 1.0);
    let nose = model.children.iter().find(|p| p.name == "nose");
    for part in std::iter::once(head).chain(nose) {
        render::draw(
            part,
            Mat4::IDENTITY,
            &Poses::default(),
            [0.0, 0.0, 1.0, 1.0],
            tint,
            render::FULL_BRIGHT,
            0,
            &mut vertices,
        );
    }
    // The front of the model faces -Z. Paint its features in depth order.
    let mut faces: Vec<_> = vertices
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|q| q[0].normal[2] < -8)
        .collect();
    let depth = |q: &[Vertex; 4]| q.iter().map(|v| v.pos[2]).sum::<f32>();
    faces.sort_by(|a, b| depth(b).total_cmp(&depth(a)));
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for face in &faces {
        for vertex in *face {
            for axis in 0..2 {
                low[axis] = low[axis].min(vertex.pos[axis]);
                high[axis] = high[axis].max(vertex.pos[axis]);
            }
        }
    }
    let size = (high[0] - low[0]).max(high[1] - low[1]);
    if !size.is_finite() || size <= 0.0 {
        return false;
    }
    let [cx, cy] = at.map(f32::round);
    let [mx, my] = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
    let scale = MARKER_SIZE / size;
    let eyes = render::layers(mob, mob.age as f32)
        .into_iter()
        .find(|layer| layer.full_bright && layer.model == key)
        .and_then(|layer| atlas.sprites.get(layer.texture.as_ref()).copied());
    for face in faces {
        let position = face.map(|v| [cx + (v.pos[0] - mx) * scale, cy + (v.pos[1] - my) * scale]);
        let uv = |r: Rect| {
            face.map(|v| {
                [
                    r.u0 + (r.u1 - r.u0) * v.uv[0],
                    r.v0 + (r.v1 - r.v0) * v.uv[1],
                ]
            })
        };
        batch.quad(position, uv(sprite), tint);
        if let Some(eyes) = eyes {
            batch.quad(position, uv(eyes), u32::MAX);
        }
    }
    true
}
