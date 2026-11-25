# Atlas native mod ports

The launcher offers optional native implementations based on the references
listed in [MOD-PORT-NOTICES.md](MOD-PORT-NOTICES.md).
Java JARs are reference material; Atlas never executes them or includes them in builds.
Exact reference versions, source hashes, authors and licenses are recorded in
[MOD-PORT-NOTICES.md](MOD-PORT-NOTICES.md).

## Existing ports

| Mod | Native behavior |
| --- | --- |
| AppleSkin 3.0.10 | Saturation quarters, exhaustion underlay, held-food/hunger/healing previews, offhand support, tooltips, original configuration and server synchronization |
| ArmorIndicator 1.0.0 | Target armor HUD, two-second retention, durability numbers/percentages/bars, damaged-equipment overlay |
| AutoSprint | Original forward/sneaking/swimming/food conditions; vanilla remains authoritative |
| NoShakeHand 1.0.0 | Separate camera sway, hand bobbing and map-pitch settings; toggle/reload commands |
| Zoomify 2.15.2 | Geometric wheel tiers, smooth entry/exit, hold/toggle and secondary zoom, retained tiers, sensitivity, hand FOV and spyglass behavior |
| MentionSound 1.0.0 | Case-insensitive substring mentions, extra names, configurable 500-ms default cooldown, own-message exclusion and WAV/AIFF notification files |

## Additional implementations

| Mod | Native behavior |
| --- | --- |
| Radar / BetterNear | Nearby-player names, health, direction, distance and heads |
| Crystal Optimizer | Client-side crystal removal after attack and crystal-use delay reset |
| NoFluid | Independent fluid-fog, water/fire-overlay and water-vision switches |
| PvPSurvived | Original combat-end messages, boss-bar tracking, cooldown and sound |
| TotemPops | Player pop counters, reset rules, chat notices and world-space labels |
| AutoCraft | Original crystal/eye/arrow crafting click sequence and feedback |
| DropAll | Original player-inventory and chest drop buttons/slot ranges |
| MiddleClickPearl | Temporary hotbar selection, pearl use, restoration and pick-block suppression |
| SortMod / Chest Sorter | Original chest sorting click schedule |
| StreamerMode | Auth-command password masking, protected history and suggestions |
| Chunk Animator | Five section-entry modes, eleven easings and first-visible-draw timing |
| Gamma Utils | Gamma/night-vision controls, transitions, dynamic light, dimensions and optional client-only status indicators |
| Aspect Ratio | H opens the original live slider and five presets; Reset restores the window ratio. The world and hand stretch while the HUD stays at the window size. |
| ESC Music Controller | Pause menu playback controls, title, artist, cover and timeline from the current Windows media session. |
| Saturation Plus 1.3.1-1.21.2 | Five hunger and exhaustion game rules, saturation commands, and the server-synchronized numerical HUD. |
| Mouse Tweaks 2.27 | Inventory mouse-drag and wheel behavior using the official Java release as reference. Settings are stored in `config/MouseTweaks.cfg`. |
| Totem Tweaks 1.0.1 | Held totem sizing, configurable pop animation, inventory totem count and optional color overlay. The port uses Atlas's configuration screen. |
| In-Game Account Switcher 9.0.7 | Title and multiplayer account controls use the launcher's shared account store and switch the client session in place. |

## Configuration and validation

The built-in Mod Menu 21.0.0 is available from the title and pause menus. Its
original two-pane browser lists installed native/content mods, search, metadata,
filters and available native configuration callbacks. Its 21 visible preferences
and file-only options use per-instance `config/modmenu.json`. Launcher installation
and enable controls remain separate from browsing. An unavailable Java configuration
plugin does not acquire a fake settings button. See MOD-PORT-NOTICES.md for the
reference and the native import/update boundaries.

`atlas-ports.json` records installation separately in each instance;
`atlas-mods.txt` records enable flags and key bindings. Installation/removal
applies at the next game launch; in-game toggles apply immediately. Nothing is
installed by default. Additional windows copy the listed mod configuration,
never saved worlds or account credentials.

The port-owned screens follow the supplied Java versions: AppleSkin and Chunk
Animator use Cloth-style configuration; Gamma Utils keeps its three Cloth
categories; Zoomify keeps its four YACL categories. ArmorIndicator and Radar
retain their custom screens.
Ports with no original configuration GUI do not expose the generic Atlas JSON
editor through Mod Menu. Original mod screens use their original bitmap font;
ordinary Atlas game screens retain the smooth font.
Native mod key bindings remain available in the vanilla Controls / Key Binds list,
including ports that have no configuration screen.

Two differences were explicitly approved by SFrostStar: Zoomify Check Migrations
is visible but disabled with an explanation until its Java source mods are ported;
Unbind Conflicting removes other bindings matching the primary zoom key, even
though the supplied reference JAR produced no change in the controlled Java run.
Native UI validation passed 31 scenarios at 1280x720: English at GUI scale 2 and
Russian at GUI scale 3, including opening configuration before entering a world,
save/cancel/reset, RGB editing, all Zoomify presets and native key rebinding.
Further GUI scales and uncommon interaction combinations still require validation;
this is not a blanket claim of pixel-for-pixel equivalence.

Checks available in this repository:

- `python scripts/check-mod-port-models.py`: native model tests.
- `node --test tests/*.test.cjs`: launcher/session/catalog tests.
- `node scripts/verify-mod-ports.cjs RUNTIME`: isolated native gameplay checks.
- `node scripts/verify-appleskin.cjs RUNTIME`: AppleSkin HUD regression.

Known boundaries: Java Fabric/Cloth/REI/JEI plugin APIs are not native APIs;
server-specific automation still requires validation against the target server.
MentionSound's AU files and asynchronous fallback after a malformed custom sound
are not yet reproduced.

AppleSkin uses `config/appleskin.json5` and the original payloads
`appleskin:saturation`, `appleskin:exhaustion`, and
`appleskin:natural_regeneration`. The integrated Atlas server synchronizes them;
external-server exhaustion requires a compatible server plugin, as in Java.

## Updating the engine

The feature code stays in `crates/atlas-client/src/game/atlas_ports/` and
renderer-owned helpers in `crates/atlas-client-render/src/atlas/`.
`scripts/export-atlas-module.cjs` exports these together with integration
references, translations, licenses and the upstream revision. Adapter files
still require review during an upstream merge; this is not a binary plugin or
an automatic guarantee that an engine update preserves every hook.

Horizon registry entries remain lazy-loaded and compatibility checked. Original
`.hnmod` code packages require a separate source/build adapter; downloading a
catalog entry never pretends to install incompatible code.
