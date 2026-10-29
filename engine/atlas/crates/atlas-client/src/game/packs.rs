//! Resource pack selection bound to the game: the pack screen lists, and a
//! full resource reload after a change, while playing.
use super::{
    Game, PackResources, Resources, UiResources, build_renderer, prepare_renderer_textures,
};
use atlas_client::{
    assets::Assets,
    gfx::PreparedTextures,
    hud::HudAtlas,
    lang,
    pack_screen::{self, Compat, PackInfo, Source as PackSource},
    packs::{self, Pack, Position},
    streaming::WorkerContext,
    text::{Style, Text},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

const RELOAD_MARKER: &str = ".atlas-resource-reload.json";

/// Persist the last selection *before* loading untrusted pack content. A
/// process abort cannot run error handlers, so the next launch repairs it.
pub(super) fn mark_reload(previous: &[String]) -> std::io::Result<()> {
    write_reload_marker(&std::env::current_dir()?, previous)
}

fn write_reload_marker(home: &Path, previous: &[String]) -> std::io::Result<()> {
    let temporary = home.join(".atlas-resource-reload.tmp");
    std::fs::write(&temporary, serde_json::to_vec(previous)?)?;
    std::fs::rename(temporary, home.join(RELOAD_MARKER))
}

pub(super) fn clear_reload_marker() {
    if let Ok(home) = std::env::current_dir() {
        let _ = std::fs::remove_file(home.join(RELOAD_MARKER));
    }
}

fn interrupted_selection(home: &Path) -> Option<Vec<String>> {
    let marker = home.join(RELOAD_MARKER);
    if std::fs::metadata(&marker).ok()?.len() > 65_536 {
        return None;
    }
    let selection: Vec<String> = serde_json::from_slice(&std::fs::read(marker).ok()?).ok()?;
    (selection.len() <= 256 && selection.iter().all(|name| name.len() <= 512)).then_some(selection)
}

pub(super) fn recover_interrupted_reload(settings: &mut atlas_client::settings::Settings) {
    let Ok(home) = std::env::current_dir() else {
        return;
    };
    if let Some(selection) = interrupted_selection(&home) {
        settings.resource_packs = selection;
        settings.save(&home.join("options.txt"));
        clear_reload_marker();
        log::warn!("Atlas: restored the last working resource packs after an interrupted reload");
    }
}

/// Where overlay roots are built (beside options.txt).
pub fn cache_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join(".atlas-cache/resourcepacks")
}
pub fn packs_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("resourcepacks")
}
/// `Util.getPlatform().openPath`: the system file manager on the folder.
pub fn open_folder(dir: &Path) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(dir).spawn() {
        log::warn!("Atlas: could not open {}: {e}", dir.display());
    }
}
/// `Util.getPlatform().openUri`: the system browser on the link.
pub fn open_uri(uri: &str) {
    open_folder(Path::new(uri));
}
/// The resource root to load from, given the vanilla root, the selection and
/// a server's packs, which go on top (`Pack.Position.TOP`, fixed).
pub fn startup_root(
    vanilla: &Path,
    selected: &[String],
    server: &[Pack],
) -> std::io::Result<PathBuf> {
    let all = packs::discover(vanilla, &packs_dir());
    let mut enabled = packs::enabled(&all, selected);
    enabled.extend_from_slice(server);
    packs::resource_root(vanilla, &enabled, &cache_dir())
}
fn info(p: &Pack) -> PackInfo {
    let built_in = !p.id.starts_with("file/");
    PackInfo {
        id: p.id.clone(),
        title: p.title.clone(),
        description: p.raw_description.clone(),
        description_style: p.description_style.clone(),
        source: if built_in {
            PackSource::BuiltIn
        } else {
            PackSource::Default
        },
        compat: p.compatibility,
        icon: p.icon_png().as_deref().and_then(pack_screen::load_icon),
        required: false,
        position: Position::Top,
        fixed: false,
    }
}
/// The vanilla pack: required, at the bottom (`BuiltInPackSource`), with the
/// client JAR's `pack.png` that the prepared resources hold.
fn vanilla_info(vanilla: &Path) -> PackInfo {
    PackInfo {
        id: "vanilla".into(),
        title: Text::translate("resourcePack.vanilla.name", &[]),
        description: Text::translate("resourcePack.vanilla.description", &[]),
        description_style: Style::default(),
        source: PackSource::BuiltIn,
        compat: Compat::Compatible,
        icon: std::fs::read(vanilla.join("pack.png"))
            .ok()
            .as_deref()
            .and_then(pack_screen::load_icon),
        required: true,
        position: Position::Bottom,
        fixed: false,
    }
}
/// Shared by the startup menu, which has no world resources yet.
pub(super) fn pack_infos(vanilla: &Path) -> Vec<PackInfo> {
    let mut infos: Vec<PackInfo> = packs::discover(vanilla, &packs_dir())
        .iter()
        .map(info)
        .collect();
    infos.push(vanilla_info(vanilla));
    infos
}
impl Game<'_> {
    pub(super) fn vanilla_root(&self) -> PathBuf {
        if self.ui.vanilla_root.as_os_str().is_empty() {
            self.options.resources.clone()
        } else {
            self.ui.vanilla_root.clone()
        }
    }
    /// The packs the pack screen offers and the current selection
    /// (`PackSelectionScreen` makes its `PackSelectionModel` from them).
    pub(super) fn refresh_pack_lists(&mut self) {
        let infos = pack_infos(&self.vanilla_root());
        let o = &mut self.ui.options;
        o.pack_infos = infos;
        o.pack_selection = self.settings.resource_packs.clone();
        o.pack_screen = None;
    }
    /// `Minecraft.reloadResourcePacks`: rebuild the overlay and reload every
    /// client resource from it, as vanilla's reload listeners do: block
    /// models and sprites (every section is meshed again,
    /// `LevelRenderer.allChanged`), the shared atlas (GUI, item, entity,
    /// particle and font textures), item models, sounds, languages, GUI
    /// metadata and the cloud and world border textures.
    pub(super) fn reload_packs(&mut self) {
        // Pack reload updates tail-owned mob metadata. Keep menus responsive
        // until those resources exist, then apply the latest selection once.
        if self.rest_loader.is_some() || self.world_loader.is_some() {
            self.pending_pack_reload = true;
            return;
        }
        if self.server_packs.reload.is_some() {
            self.pending_pack_reload = true;
            return;
        }
        self.start_resource_reload(true);
    }
    /// The render-thread half: the renderer on the new pack's textures, then
    /// the swap (`ReloadInstance`'s application listeners).
    pub(super) fn apply_staged_pack(&mut self, staged: StagedPack) -> atlas_client::Result<()> {
        let StagedPack {
            root,
            gui,
            pack,
            textures,
            renderer: prepared_renderer,
        } = staged;
        let renderer = if let Some(mut renderer) = prepared_renderer {
            if let Some(previous) = &mut self.renderer {
                if let Err(error) = renderer.attach_replacement(previous, self.options.offscreen) {
                    previous.recreate_surface()?;
                    return Err(error);
                }
            }
            Some(renderer)
        } else {
            // Deterministic/offscreen callers can still apply a staged CPU pack.
            if let Some(renderer) = &mut self.renderer {
                renderer.release_surface();
            }
            match self
                .window
                .clone()
                .map(|window| {
                    build_renderer(
                        window,
                        self.options.offscreen,
                        &pack,
                        &gui,
                        self.renderer.as_ref(),
                        textures,
                        self.settings.atlas.api,
                        None,
                    )
                })
                .transpose()
            {
                Ok(renderer) => renderer,
                Err(error) => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.recreate_surface()?;
                    }
                    return Err(error);
                }
            }
        };
        // Everything loaded: switch over.
        self.waypoint_styles = atlas_client::locator_bar::WaypointStyles::load(&root);
        self.armor_stand_assets.reload_trims(&root);
        self.random_tick_sections.clear();
        self.portals.sections.clear();
        self.options.resources = root.clone();
        let context = pack.context.clone();
        self.pack = pack;
        self.gui = gui;
        // The atlas's sprites moved: the particle engine re-reads every rect
        // and the packs' sprite sets, or every particle draws a stale region.
        self.particles
            .reload(&root, &self.gui.atlas.sprites, &self.gui.atlas.translucent);
        atlas_client::ui_theme::reset_atlas();
        // Villager hats come from the pack's texture metadata (shared data, swapped in place).
        *self
            .res
            .mob_data()
            .villager_hats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            atlas_client::mobs::villager_hats::VillagerHats::load(&root);
        atlas_client::ui::load_metadata(&root);
        lang::set(lang::Language::load(&[&root], &lang::code()));
        self.refresh_remote_language();
        self.audio.reload(&root);
        self.clear_lod();
        self.streaming.reload_resources(&mut self.world, context);
        self.pending_uploads.clear();
        // New GPU textures: a renderer built from the reloaded resources; the
        // sections are uploaded again as they are meshed.
        if let Some(renderer) = renderer {
            self.renderer = Some(renderer);
            self.apply_ui_settings();
        }
        self.ui.chat.rescale(&self.gui.atlas);
        // The creative screen survives the reload; item names may not.
        self.creative_screen.forget_items();
        Ok(())
    }
}

/// The CPU half of a resource reload (`PackReload` runs it on a
/// worker; `apply_staged_pack` does the GPU half and the swap): the overlay
/// root, the models and sprites, the menu bundle, the pack's tables and its
/// CPU mip chains.
pub(super) fn stage_pack(
    vanilla_root: &Path,
    selected: &[String],
    server: &[Pack],
    mods_enabled: &BTreeSet<String>,
    dev_mods: &[PathBuf],
    assets: &Arc<Assets>,
    res: &Resources,
    context: &WorkerContext,
    force_unicode: bool,
    japanese: bool,
    progress: &super::Progress,
) -> atlas_client::Result<StagedPack> {
    let reload_started = std::time::Instant::now();
    log::info!("Atlas: pack reload preparing overlay");
    let root = startup_root(vanilla_root, selected, server)?;
    log::info!(
        "Atlas: pack reload overlay ready in {} ms",
        reload_started.elapsed().as_millis()
    );
    let root = super::mods::content_root(&root, mods_enabled, dev_mods);
    progress.set(1);
    let render = Arc::new(assets.reload(&root)?);
    let mut atlas = HudAtlas::build_over(&root, vanilla_root)?;
    atlas.font.uniform = force_unicode;
    atlas.font.japanese = japanese;
    let gui = UiResources {
        atlas,
        panorama: Arc::new(atlas_client::gfx::panorama::Faces::load(&root)),
    };
    log::info!(
        "Atlas: pack reload GUI ready in {} ms",
        reload_started.elapsed().as_millis()
    );
    progress.set(2);
    let pack = PackResources::load(
        &root,
        assets.clone(),
        render,
        res.facts(),
        context.biomes.clone(),
        context.light.clone(),
    )?;
    let textures = prepare_renderer_textures(&pack);
    log::info!(
        "Atlas: pack reload CPU ready in {} ms",
        reload_started.elapsed().as_millis()
    );
    progress.set(3);
    Ok(StagedPack {
        root,
        gui,
        pack,
        textures: Some(textures),
        renderer: None,
    })
}

/// Everything a pack reload builds on the CPU (`apply_staged_pack` finishes
/// it on the render thread).
pub(super) struct StagedPack {
    pub root: PathBuf,
    pub gui: UiResources,
    pub pack: PackResources,
    pub textures: Option<PreparedTextures>,
    pub renderer: Option<atlas_client::gfx::Renderer>,
}

#[cfg(test)]
mod reload_recovery_tests {
    use super::*;
    #[test]
    fn interrupted_reload_retains_the_previous_selection() {
        let home = std::env::temp_dir().join(format!(
            "atlas-pack-recovery-{}",
            atlas_client::save::random_uuid()
                .iter()
                .map(|n| n.to_string())
                .collect::<String>()
        ));
        std::fs::create_dir_all(&home).unwrap();
        assert!(interrupted_selection(&home).is_none());
        let selection = vec!["vanilla".to_owned(), "file/working.zip".to_owned()];
        write_reload_marker(&home, &selection).unwrap();
        assert_eq!(interrupted_selection(&home), Some(selection));
        std::fs::remove_file(home.join(RELOAD_MARKER)).unwrap();
        std::fs::remove_dir(home).unwrap();
    }
    #[test]
    fn corrupt_marker_is_not_a_resource_selection() {
        let home = std::env::temp_dir().join(format!("atlas-pack-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(RELOAD_MARKER), b"{\"packs\":null}").unwrap();
        assert!(interrupted_selection(&home).is_none());
        std::fs::remove_file(home.join(RELOAD_MARKER)).unwrap();
        std::fs::remove_dir(home).unwrap();
    }
}
