# Atlas native mod ports

The launcher offers 26 optional native implementations: the existing six ports
and twenty additional mods inspected from SFrostStar's supplied JARs.
Java JARs are reference material and are never executed or included in builds.
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

## Twenty additional implementations

| Mod | Native behavior |
| --- | --- |
| Radar / BetterNear | Nearby-player names, health, direction, distance and heads |
| Crystal Optimizer | Client-side crystal removal after attack and crystal-use delay reset |
| FriendList | Persistent friend commands, attack protection and translucent friend models |
| KillNotification | Original kill-message parsing and queued chat notification |
| Lootbeams | Item filters, rarity outlines and rotating world-space beams |
| NoFluid | Independent fluid-fog, water/fire-overlay and water-vision switches |
| PvPSurvived | Original combat-end messages, boss-bar tracking, cooldown and sound |
| RwJoiner | ReallyWorld menu/scoreboard state machine and client commands |
| TotemPops | Player pop counters, reset rules, chat notices and world-space labels |
| CullParticles | The supplied JAR disables push-out-of-blocks; its name does not describe particle culling |
| AutoCraft | Original crystal/eye/arrow crafting click sequence and feedback |
| DropAll | Original player-inventory and chest drop buttons/slot ranges |
| HotCommands | Nested command buttons and command editor |
| Inventory Layouts | Saved 41-slot layouts and native menu-click planner |
| MiddleClickPearl | Temporary hotbar selection, pearl use, restoration and pick-block suppression |
| SortMod / Chest Sorter | Original chest sorting click schedule |
| StreamerMode | Auth-command password masking, protected history and suggestions |
| ViewModel | Separate hand transforms, thirteen swing styles, duration and interactive chat controls |
| Chunk Animator | Five section-entry modes, eleven easings and first-visible-draw timing |
| Gamma Utils | Gamma/night-vision controls, transitions, dynamic light, dimensions and optional client-only status indicators |

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

Radar and Gamma Utils deliberately use Atlas configuration screens, as approved
by SFrostStar. Further work on complex mod interfaces is deferred at the user's
request. Existing interface code is preserved; it is not a claim of complete
pixel-for-pixel visual parity. Remaining interface comparisons include all GUI
scales, HotCommands editor details and Inventory Layouts wrapped status text.

Checks available in this repository:

- `python scripts/check-mod-port-models.py`: 27 native model tests.
- `node --test tests/*.test.cjs`: launcher/session/catalog tests.
- `node scripts/verify-mod-ports.cjs RUNTIME`: isolated native gameplay checks.
- `node scripts/verify-appleskin.cjs RUNTIME`: AppleSkin HUD regression.

Additional local reference comparisons matched 160 complete Inventory Layouts
click/state scenarios and 390 ViewModel pose traces against the supplied Java
implementations. These counts do not imply every original mod feature has been
exercised end to end.

Known boundaries: Java Fabric/Cloth/REI/JEI plugin APIs are not native APIs;
server-specific automation still requires validation against the target server.
MentionSound's AU files and asynchronous fallback after a malformed custom sound
are not yet reproduced. Lootbeams' exact translucent render-state parity needs
further verification. Keep these limitations visible rather than claiming all
ports are fully identical.

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
