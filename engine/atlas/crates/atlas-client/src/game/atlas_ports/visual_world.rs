//! World-space name tags for TotemPops.
use super::{Game, TOTEM_POPS, config, logic};
use atlas_client::{
    gfx::WORLD_TEXT_OVERLAY,
    hud::HudBatch,
    mesher::Vertex,
    text::{Style, Text},
};
use glam::DVec3;

impl Game<'_> {
    /// Call beside name_tag_geometry, after entities, with the same two destination passes.
    pub(in crate::game) fn atlas_visual_world_geometry(
        &mut self,
        camera: DVec3,
        partial: f32,
        out: &mut Vec<Vertex>,
        through: &mut Vec<Vertex>,
    ) {
        if !self.native_window() || self.player.is_none() {
            return;
        }
        if self.atlas_port_on(TOTEM_POPS) && self.atlas_ports.visual.totem.enabled {
            self.atlas_visual_pop_geometry(camera, partial, out, through)
        }
    }
    fn atlas_visual_pop_geometry(
        &mut self,
        camera: DVec3,
        partial: f32,
        out: &mut Vec<Vertex>,
        through: &mut Vec<Vertex>,
    ) {
        let c = self.atlas_ports.visual.totem.clone();
        let me = self.atlas_visual_own_uuid();
        let own_name = self.own_profile_name();
        let mut players = self.remote_player_names();
        if c.show_self && !self.camera_type.first_person() && !self.spectator() {
            players.push((0, own_name.clone()))
        }
        let (yaw, pitch) = self.posed_rotation.unwrap_or_else(|| {
            if self.camera_type == super::super::super::perspective::CameraType::ThirdPersonFront {
                (self.yaw + 180., -self.pitch)
            } else {
                (self.yaw, self.pitch)
            }
        });
        let forward = DVec3::from_array(super::super::super::view_vector(yaw, pitch));
        let right = forward.cross(DVec3::Y).normalize_or_zero().as_vec3();
        let up = right
            .as_dvec3()
            .cross(forward)
            .normalize_or_zero()
            .as_vec3();
        let my_team = self
            .score
            .board
            .players_team(&own_name)
            .map(|t| t.name.clone());
        for (id, name) in players {
            let is_self = id == 0;
            let m = if is_self {
                self.player_model.as_ref()
            } else {
                self.mobs.iter().find(|m| m.id == id)
            };
            let Some(m) = m else { continue };
            if c.hide_teammates
                && !is_self
                && my_team.as_ref().is_some_and(|team| {
                    self.score
                        .board
                        .players_team(&name)
                        .is_some_and(|t| &t.name == team)
                })
            {
                continue;
            }
            // Avatar's spectator flag comes from the same player list as its rendered model.
            if !is_self && m.extra.spectator {
                continue;
            }
            let uuid = if is_self {
                me
            } else {
                self.remote_player_uuid(id).unwrap_or(id as u128)
            };
            let count = self.atlas_ports.visual.pops.count(uuid);
            if count == 0 && !c.show_zero {
                continue;
            }
            let position = m.previous.lerp(m.position, partial as f64);
            let pos = DVec3::new(position.x, position.y, position.z);
            if pos.distance_squared(camera) > c.max_distance * c.max_distance {
                continue;
            }
            let origin = (pos - camera + DVec3::Y * (f64::from(m.height()) + 0.5)).as_vec3();
            let unit = 0.025 * c.scale;
            let text = pop_text(&c, count);
            let append = |dest: &mut Vec<Vertex>, text_alpha: u8, background: bool| {
                let mut batch = HudBatch::new([1, 1]);
                batch.name_tag(
                    &self.gui.atlas,
                    &text,
                    c.y_offset,
                    (text_alpha as u32) << 24 | 0xffffff,
                    if background && c.background {
                        0x40000000
                    } else {
                        0
                    },
                );
                for q in batch.vertices.as_chunks::<6>().0 {
                    for vertex in [q[5], q[2], q[1], q[0]] {
                        dest.push(Vertex {
                            pos: (origin + right * (vertex.pos[0] * unit)
                                - up * (vertex.pos[1] * unit))
                                .to_array(),
                            uv: vertex.uv,
                            color: vertex.color,
                            layer: WORLD_TEXT_OVERLAY,
                            light: 0xf0f0,
                            normal: [0; 4],
                            glint_uv: [0.; 2],
                        })
                    }
                }
                batch.patches
            };
            let back = append(through, 0x20, true);
            let front = append(out, 0xff, false);
            self.map_patches.extend(back);
            self.map_patches.extend(front);
        }
    }
}
fn pop_text(c: &config::Totem, count: u32) -> Text {
    let count_color = if c.dynamic_color {
        logic::totem_color(count)
    } else {
        c.count_color
    };
    let mut out = Text::empty();
    let mut remaining = c.format.as_str();
    while !remaining.is_empty() {
        if let Some(tail) = remaining.strip_prefix("{label}") {
            out.push(Text::colored(&c.label, c.label_color));
            remaining = tail
        } else if let Some(tail) = remaining.strip_prefix("{count}") {
            out.push(Text::colored(
                format!("{}{count}", c.count_prefix),
                count_color,
            ));
            remaining = tail
        } else {
            let size = remaining.chars().next().unwrap().len_utf8();
            out.push(Text::styled(
                &remaining[..size],
                Style {
                    color: Some(c.label_color),
                    ..Default::default()
                },
            ));
            remaining = &remaining[size..]
        }
    }
    out
}
