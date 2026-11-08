# Atlas optional mod ports — provenance and notices

The optional native modules are based on the supplied Java mod versions below.
Authorship and declared licenses belong to the original authors; the Atlas adapters
and their integration are maintained by SFrostStar. Mod names are attribution,
not a claim of affiliation or endorsement. Original Java JARs are not bundled.

For AppleSkin 3.0.10 (squeek502, Unlicense), see the existing AppleSkin section in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the original notice beside its icons.

## Reference versions and authors

The version is the actual `fabric.mod.json` value, even when a filename says something else.
`${version}` in AutoSprint is an unresolved value in the supplied JAR; no release number is invented.

| Reference JAR | Metadata version | Original author(s) | Metadata license | Embedded notice |
| --- | --- | --- | --- | --- |
| `ArmorIndicator-1.0.0.jar` | `1.0.0` | ArmorIndicator Dev | MIT | CC0-1.0 |
| `AutoSprint-1.21.4-Fabric.jar` | `${version}` | Manus | MIT | not supplied |
| `NoShakeHand-1.0.0.jar` | `1.0.0` | slmnk | MIT | MIT |
| `zoomify-2.15.2+1.21.5.jar` | `2.15.2+1.21.5` | isXander | LGPL-3.0-or-later | not supplied |
| `mentionsound-1.0.0.jar` | `1.0.0` | You | MIT | not supplied |
| `betternear-1.0.0.jar` | `1.0.0` | slmnk | MIT | not supplied |
| `crystaloptimizer-1.0.0.jar` | `1.0.0` | Author | MIT | not supplied |
| `friendlist-1.0.0.jar` | `1.0.0` | slmnk | MIT | not supplied |
| `killnotification-1.0.0.jar` | `1.0.0` | Slmnk | MIT | not supplied |
| `lootbeams-2.1.0.jar` | `1.1.0` | you | CC0-1.0 | not supplied |
| `nofluid-1.21.4.jar` | `1.0.0` | slmnk | MIT | CC0-1.0 |
| `pvpsurvived-1.0.0.jar` | `1.0.0` | Slmnk | MIT | CC0-1.0 |
| `RwJoiner-fabric-1.21.4-1.0.0.jar` | `1.0.0` | RwJoiner Team | MIT | not supplied |
| `totempops-1.0.0.jar` | `1.0.0` | you | MIT | not supplied |
| `CullParticles-1_21_4_1_0_0.jar` | `1.21.4_1.0` | quikjay | MIT | not supplied |
| `autocraft-1.0.0.jar` | `1.0.0` | AutoCraft Developer | MIT | CC0-1.0 |
| `dropall-2.0.0.jar` | `1.0.0` | Author | MIT | not supplied |
| `hotcommands-1.0.0.jar` | `1.0.0` | Slmnk | MIT | not supplied |
| `inventory-manager-1.01.jar` | `1.01` | Inventory Layouts | MIT | not supplied |
| `middleclickpearl-1.0.0.jar` | `1.0.0` | you | CC0-1.0 | CC0-1.0 |
| `sortmod-3.0.0.jar` | `1.0.0` | Developer | MIT | not supplied |
| `streamermode-1.0.0.jar` | `1.0.0` | StreamerMode | MIT | CC0-1.0 |
| `ViewModel-1.21.4.jar` | `1.0.0` | ViewModel | MIT | CC0-1.0 |
| `chunkanimator-1.21.4.jar` | `1.0.0+1.21.4` | Donyka | MIT | not supplied |
| `Gamma-Utils-2.2.6+mc1.21.5 (1).jar` | `2.2.6` | Sjouwer | GNU LGPLv3 | LGPL-3.0 |

A number of JARs declare MIT in metadata but contain a CC0 notice; both pieces
of provenance are recorded above and the original embedded text is retained verbatim.
A missing notice is not replaced with an invented copyright year or an invented author.

## Retained license texts

The following files are copied byte-for-byte from the supplied archives:

- [armorindicator-LICENSE_ArmorIndicator.txt](mod-port-licenses/armorindicator-LICENSE_ArmorIndicator.txt) — armorindicator, CC0-1.0.
- [noshakehand-LICENSE_NoShakeHand.txt](mod-port-licenses/noshakehand-LICENSE_NoShakeHand.txt) — noshakehand, MIT.
- [nofluid-LICENSE_nofluid.txt](mod-port-licenses/nofluid-LICENSE_nofluid.txt) — nofluid, CC0-1.0.
- [pvpsurvived-LICENSE_pvpsurvived.txt](mod-port-licenses/pvpsurvived-LICENSE_pvpsurvived.txt) — pvpsurvived, CC0-1.0.
- [autocraft-LICENSE_autocraft.txt](mod-port-licenses/autocraft-LICENSE_autocraft.txt) — autocraft, CC0-1.0.
- [middleclickpearl-LICENSE_middleclickpearl.txt](mod-port-licenses/middleclickpearl-LICENSE_middleclickpearl.txt) — middleclickpearl, CC0-1.0.
- [streamermode-LICENSE_streamermode.txt](mod-port-licenses/streamermode-LICENSE_streamermode.txt) — streamermode, CC0-1.0.
- [viewmodel-LICENSE_viewmodel.txt](mod-port-licenses/viewmodel-LICENSE_viewmodel.txt) — viewmodel, CC0-1.0.
- [gammautils-LICENSE_Gamma-Utils.txt](mod-port-licenses/gammautils-LICENSE_Gamma-Utils.txt) — gammautils, LGPL-3.0.

[MIT reference terms](mod-port-licenses/MIT-reference.txt) accompany the metadata-only MIT modules.
The MIT notice with the original `Copyright (c) 2026 slmnk` for NoShakeHand is preserved separately.

Zoomify and Gamma Utils are LGPL references. Atlas uses independently written native
implementations of their behavior and records their original license and authorship;
the references must not be described as MIT-licensed upstream source. Their original projects are
[Zoomify](https://github.com/isXander/Zoomify) and [Gamma Utils](https://github.com/Sjouwer/gamma-utils).

## Changes in Atlas

- Fabric callbacks, mixins, Brigadier commands and rendering calls are adapted to the native Rust client.
- Native game data and per-instance configuration replace Java runtime objects.
- HUD positions, colors, constants, food/equipment calculations and state timers are taken from the supplied versions.
- Atlas intentionally uses a shared configuration editor while preserving the mod mechanics and persisted JSON/YAML settings. This chosen settings interface differs from RadarConfigScreen and the Java configuration libraries.
- RwJoiner retains its ReallyWorld-specific menu/scoreboard assumptions, including the original hardcoded `11` material exception.
- Lootbeams uses vanilla item silhouettes. Its original `lineWidth`, `expand`, `visibleAlpha` and `xrayAlpha` settings were already unused by the supplied renderer and are retained without invented behavior.
- The supplied CullParticles JAR contains only a mixin cancelling `ClientPlayerEntity.pushOutOfBlocks(DD)V`; it does not cull particles. Atlas gates the corresponding `closest_space_nudge` push-out step. This entry must not be advertised as particle optimization.
- The native Mod Menu displays the original supplied icons for AppleSkin, ArmorIndicator, AutoCraft, Chunk Animator, Crystal Optimizer, CullParticles, Gamma Utils, NoFluid, NoShakeHand, PvPSurvived, Radar/BetterNear, RwJoiner, StreamerMode, ViewModel and Zoomify. They retain the corresponding reference authorship and licenses listed above; AppleSkin has its separate retained notice. Mods without an original icon use the menu's fallback image. Original MIT English/Russian localization entries from HotCommands (Slmnk) and Inventory Layouts are adapted into Atlas's native language bundle.
- Gamma Utils' optional custom status-effect icon is not reproduced in the current independent native adapter; brightness control must not be presented as complete parity with that additional display feature.

No blanket claim of visual or behavior equivalence is made without in-game comparison.
The chosen configuration-screen layout is recorded separately from gameplay and HUD equivalence.
Automated tests cover numerical/state boundaries; renderer and multiplayer behavior still require the checks recorded in ATLAS-PORTS.md.

## Mod Menu 21.0.0

The supplied `modmenu-21.0.0-26.3-fabric.jar` is the reference for the built-in native menu.
SHA-256: `837812c6c98d66141db7b5dcae668f27fb78d557e8f4c0c89912985fc26a5e6f`.
Original project: [TerraformersMC/ModMenu](https://github.com/TerraformersMC/ModMenu).
Copyright (c) 2018–2020 Prospector, MIT. The original complete notice is retained in
`crates/atlas-client-render/src/atlas/mod_menu/LICENSE-ModMenu.txt` and accompanies the runtime.

Atlas adapts the original configuration defaults, search weighting, stable sorting,
two-pane layout, navigation, filter/configuration controls, localization and bundled
Mod Menu sprites to Rust. Native registry metadata and configuration callbacks replace
Fabric APIs. Mod Menu is built in; neither the reference JAR nor Fabric is distributed.
The menu's configuration is saved per instance in `config/modmenu.json`.

Java-only config plugins and arbitrary Java mod update/install APIs are not executable
in Atlas. Configuration buttons are shown only for available native configuration
callbacks. Compatible `.hmod`/`.hnmod` imports are validated; `.hnmod` still needs a native
build adapter. No Fabric dependencies or Java update availability are fabricated.

## Source archive fingerprints

These hashes identify the exact local reference archives, without uploading the archives or player data.

| Reference | SHA-256 |
| --- | --- |
| `ArmorIndicator-1.0.0.jar` | `6ba9dbae528902568e06a4ab40ca17f07a3e5e184a0584d2baebed213ef8952b` |
| `AutoSprint-1.21.4-Fabric.jar` | `860e3fe65289193a48c12bc186965e7be93951e12f32f211a41a9756d72ec05a` |
| `NoShakeHand-1.0.0.jar` | `592ee550f3c4e71def7074df85bdf5b22a870d4a144ea285af302e2563568b5e` |
| `zoomify-2.15.2+1.21.5.jar` | `cd7366993960c9b104c3f224bc8d9f3e3b63840e3b76535ebcda2271e002791e` |
| `mentionsound-1.0.0.jar` | `bf98f3786a1c5bec7a26366da90df6734c2a38aa4bdcd844f61850985d5aac74` |
| `betternear-1.0.0.jar` | `f70bff83b74038fb679c5565e60d5498e626d4e1a46b5ba75a318aa6f5288712` |
| `crystaloptimizer-1.0.0.jar` | `078f3a65027f6ad8562f60f9d793484d753c252916397237ce12867544c25b76` |
| `friendlist-1.0.0.jar` | `d4a7f37f63a452073e879829311c2791f0b1a0f4344eb6e5e9a6a8dcb731eeb9` |
| `killnotification-1.0.0.jar` | `19d9f3199c0ae99c226c4335b7baf09c6b979bf2b3847cee23757961a88b4e7d` |
| `lootbeams-2.1.0.jar` | `6900904e07f1fad2c4392020e1e89de4342c168d6d7f4f6c95c8be54ce902d6f` |
| `nofluid-1.21.4.jar` | `55829dc26a89be449fa1bb91b47d8f57e5012ec6210317fd5ea3dce8c1cf2ad4` |
| `pvpsurvived-1.0.0.jar` | `5ec09ff7a09109ed6a449b12b8a0c6b082b942b01a7ad395e191dd9030135ebd` |
| `RwJoiner-fabric-1.21.4-1.0.0.jar` | `593ab1675435191557b40b0c7f5dab7557e0067e2ad19514f5a16de536ae00ee` |
| `totempops-1.0.0.jar` | `b6b30b03a3256224c9c2698b1b27f2a7f8df949ce3000c07f61815b95ba451bd` |
| `CullParticles-1_21_4_1_0_0.jar` | `3503b616c432074415c7c7f50898e168f9ab3e7eed0efdce9f523ccc15e2ca59` |
| `autocraft-1.0.0.jar` | `a1db1a41dfd0e25508b9ba3fb1a74992c1ccf0fbadc48130d511026e1e9d1c6b` |
| `dropall-2.0.0.jar` | `882f8df3da90c84c4e23b67e086bd48aa9777418e9340b4d613c3dc57a21a7a7` |
| `hotcommands-1.0.0.jar` | `229c9caf580a7a364b0d5f0a613fda161eb869006983621b27300440eea86fb1` |
| `inventory-manager-1.01.jar` | `5317257c8d49347fd200fb7d2a18d416ec41beb78ca250ba4fe22b620b820120` |
| `middleclickpearl-1.0.0.jar` | `b059f6229c2560d7693578b65ea0f3e9f2ce66c7d120a43e7621314cff49f0af` |
| `sortmod-3.0.0.jar` | `2222626d76338a2433db36c2fed339e4d27fb20477ed8fcb9033effd654de94a` |
| `streamermode-1.0.0.jar` | `0caad369817498cce5b991df16e9d05018e77121a4a1385e20c6f809a69b4fc4` |
| `ViewModel-1.21.4.jar` | `ca1b8969ba3bd447f498b4c58abfa0a2680572ad21e1b5baf20dd659db0b460d` |
| `chunkanimator-1.21.4.jar` | `73b77c235115309c6f57ebe42275a93637ae922d2d56095be86203f9c4301520` |
| `Gamma-Utils-2.2.6+mc1.21.5 (1).jar` | `1fa56090c812a851cd603f9392372b8c945c585d0c027c5e759c774ece4cdabe` |

The reference for the unusual CullParticles method mapping is the
[Fabric Yarn 1.21.4+build.8 ClientPlayerEntity mapping](https://maven.fabricmc.net/docs/yarn-1.21.4+build.8/net/minecraft/client/network/ClientPlayerEntity.html#pushOutOfBlocks(double,double)).


## Gamma Utils artwork

The three unmodified status-effect PNGs under
`crates/atlas-client-render/src/atlas/assets/gammautils/` are from the supplied
Gamma Utils 2.2.6 JAR by Sjouwer, under GNU LGPL v3. Their original source paths
are `assets/gammautils/textures/mob_effect/{bright,dim,night_vision}.png`.
They remain separate artwork with their original license; they are not relicensed
as Atlas MIT artwork. Corresponding editable PNG source is included unchanged.
See `mod-port-licenses/LGPL-3.0-GammaUtils.txt`, `mod-port-licenses/GPL-3.0.txt`
and https://github.com/Sjouwer/gamma-utils for the original project.
