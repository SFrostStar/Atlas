# Atlas feature ports

Original Rust implementations inspired by the features of Java mods selected by SFrostStar.
No Java classes, third-party implementation code or mod artwork is included.
The implementations use Atlas's existing vanilla simulation, item properties,
rendering and native mod interfaces. They are initial feature equivalents, not
complete reproductions of the original projects.

| Port | Implemented | Not included |
|---|---|---|
| AppleSkin | Current hunger/saturation, held-food restoration preview | Exhaustion HUD, health prediction, extended tooltips and server extensions |
| ArmorIndicator | Equipped local armor, remaining/max durability, colored warnings | Other players' armor/target HUD and red entity tint |
| AutoSprint | Ordinary sprint request while holding forward, honoring vanilla movement/food checks | Advanced configuration |
| NoShakeHand | Removes inertial hand sway when turning | Does not remove walking bob, hurt, attack, throwing or item-use animations |
| Zoomify | Smooth 4× hold-to-zoom on C, rebindable in Mods, spyglass priority | Scroll zoom and the original project's extended settings |
| MentionSound | Nickname mentions in incoming visible chat, nickname boundaries, self-prefix exclusion, 40-tick cooldown | Custom sound configuration and server-specific message formats |

Code: `crates/atlas-client/src/game/atlas_ports/`. The launcher writes
`atlas-ports.json` and preserves `atlas-mods.txt` separately in each instance.
Install/uninstall and launcher toggles require the game to be closed; they apply
at the next launch. In-game native mod toggles apply through `ModList::disabled`.
Nothing is installed or enabled by default. Additional game windows copy these
settings to their isolated profile.

Horizon registry entries are fetched lazily, cached, and presented with compatibility
information. Only explicit Atlas equivalents are installable by this first adapter
(currently Zoom/Classic Zoom → the bundled Atlas zoom implementation). Original
native `.hnmod` packages need a separate source/build adapter; old WebAssembly code
mods are unsupported by the pinned engine. The catalogue never labels these
packages installed merely because their entry was downloaded.

Upstream updates must retain the `atlas_ports` state and calls in `game.rs`,
`game/remote.rs`, `game/chat.rs`, `game/extras.rs` and `game/native_hooks.rs`.
`scripts/export-atlas-module.cjs` exports the feature-port sources together with
the Atlas presentation/loading module and adapter references.
