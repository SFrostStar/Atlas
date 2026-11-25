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
| `aspectratio-1.0.0.jar` | 1.0.0 | usaryy | Unspecified | None found |
| `esc-music-controller-1.0.0.jar` | 1.0.0 | AntiJOPA | MIT | None found |
| `ArmorIndicator-1.0.0.jar` | `1.0.0` | ArmorIndicator Dev | MIT | CC0-1.0 |
| `AutoSprint-1.21.4-Fabric.jar` | `${version}` | Manus | MIT | not supplied |
| `NoShakeHand-1.0.0.jar` | `1.0.0` | slmnk | MIT | MIT |
| `zoomify-2.15.2+1.21.5.jar` | `2.15.2+1.21.5` | isXander | LGPL-3.0-or-later | not supplied |
| `mentionsound-1.0.0.jar` | `1.0.0` | You | MIT | not supplied |
| `betternear-1.0.0.jar` | `1.0.0` | slmnk | MIT | not supplied |
| `crystaloptimizer-1.0.0.jar` | `1.0.0` | Author | MIT | not supplied |
| `nofluid-1.21.4.jar` | `1.0.0` | slmnk | MIT | CC0-1.0 |
| `pvpsurvived-1.0.0.jar` | `1.0.0` | Slmnk | MIT | CC0-1.0 |
| `totempops-1.0.0.jar` | `1.0.0` | you | MIT | not supplied |
| `autocraft-1.0.0.jar` | `1.0.0` | AutoCraft Developer | MIT | CC0-1.0 |
| `dropall-2.0.0.jar` | `1.0.0` | Author | MIT | not supplied |
| `middleclickpearl-1.0.0.jar` | `1.0.0` | you | CC0-1.0 | CC0-1.0 |
| `sortmod-3.0.0.jar` | `1.0.0` | Developer | MIT | not supplied |
| `streamermode-1.0.0.jar` | `1.0.0` | StreamerMode | MIT | CC0-1.0 |
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
- The native configuration screens independently reproduce the observed original custom, Cloth and YACL layouts and their save/cancel/reset behavior. Original EN/RU labels and tooltips from AppleSkin, Zoomify, Gamma Utils, Cloth Config (shedaniel, LGPLv3) and YACL (isXander, LGPL-3.0-or-later) retain their original authorship. Full LGPL/GPL texts are retained in `mod-port-licenses/LGPL-3.0-GammaUtils.txt` and `mod-port-licenses/GPL-3.0.txt`. Java configuration libraries are not bundled or executed in Atlas.
- The native Mod Menu displays the original supplied icons for AppleSkin, ArmorIndicator, AutoCraft, Chunk Animator, Crystal Optimizer, Gamma Utils, NoFluid, NoShakeHand, PvPSurvived, Radar/BetterNear, StreamerMode and Zoomify. They retain the corresponding reference authorship and licenses listed above; AppleSkin has its separate retained notice. Mods without an original icon use the menu's fallback image.
- Gamma Utils retains its original optional status-effect artwork with separate LGPL/GPL provenance below; visual equivalence beyond the checked scenarios is not claimed.

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
| `aspectratio-1.0.0.jar` | `c946bf3c49bdfb77daf65b4dbd47dfb09f6106469c706409d85e6358a93bc073` |
| `esc-music-controller-1.0.0.jar` | `581c91c385841b8bd625c8a08ca591dbda94af2fbc652182d037e89934f03390` |
| `ArmorIndicator-1.0.0.jar` | `6ba9dbae528902568e06a4ab40ca17f07a3e5e184a0584d2baebed213ef8952b` |
| `AutoSprint-1.21.4-Fabric.jar` | `860e3fe65289193a48c12bc186965e7be93951e12f32f211a41a9756d72ec05a` |
| `NoShakeHand-1.0.0.jar` | `592ee550f3c4e71def7074df85bdf5b22a870d4a144ea285af302e2563568b5e` |
| `zoomify-2.15.2+1.21.5.jar` | `cd7366993960c9b104c3f224bc8d9f3e3b63840e3b76535ebcda2271e002791e` |
| `mentionsound-1.0.0.jar` | `bf98f3786a1c5bec7a26366da90df6734c2a38aa4bdcd844f61850985d5aac74` |
| `betternear-1.0.0.jar` | `f70bff83b74038fb679c5565e60d5498e626d4e1a46b5ba75a318aa6f5288712` |
| `crystaloptimizer-1.0.0.jar` | `078f3a65027f6ad8562f60f9d793484d753c252916397237ce12867544c25b76` |
| `nofluid-1.21.4.jar` | `55829dc26a89be449fa1bb91b47d8f57e5012ec6210317fd5ea3dce8c1cf2ad4` |
| `pvpsurvived-1.0.0.jar` | `5ec09ff7a09109ed6a449b12b8a0c6b082b942b01a7ad395e191dd9030135ebd` |
| `totempops-1.0.0.jar` | `b6b30b03a3256224c9c2698b1b27f2a7f8df949ce3000c07f61815b95ba451bd` |
| `autocraft-1.0.0.jar` | `a1db1a41dfd0e25508b9ba3fb1a74992c1ccf0fbadc48130d511026e1e9d1c6b` |
| `dropall-2.0.0.jar` | `882f8df3da90c84c4e23b67e086bd48aa9777418e9340b4d613c3dc57a21a7a7` |
| `middleclickpearl-1.0.0.jar` | `b059f6229c2560d7693578b65ea0f3e9f2ce66c7d120a43e7621314cff49f0af` |
| `sortmod-3.0.0.jar` | `2222626d76338a2433db36c2fed339e4d27fb20477ed8fcb9033effd654de94a` |
| `streamermode-1.0.0.jar` | `0caad369817498cce5b991df16e9d05018e77121a4a1385e20c6f809a69b4fc4` |
| `chunkanimator-1.21.4.jar` | `73b77c235115309c6f57ebe42275a93637ae922d2d56095be86203f9c4301520` |
| `Gamma-Utils-2.2.6+mc1.21.5 (1).jar` | `1fa56090c812a851cd603f9392372b8c945c585d0c027c5e759c774ece4cdabe` |
| `saturation_plus-1.3.1-1.21.2.jar` | `bd00f8475f935a987c040d6e2e3fadf9eb90d44316158c55b078db73b52746dc` |
| Official `MouseTweaks-fabric-mc1.21.3-2.27.jar` | `82aaa759bb702c5b958e1d8f9ae0935213e50a9588dce8be0cbfb5074121bc48` |
| `totemtweaks-1.21.4-1.0.1.jar` | `bc5b7a0e72c19b3078ac3ad21c4437f3fbf16f7420c1593f59f3b98e0e5b4324` |

Saturation Plus 1.3.1-1.21.2 is by MrKirbychu and licensed CC0-1.0. Its
original notice is in `mod-port-licenses/saturation_plus-LICENSE_saturation_plus.txt`.

Mouse Tweaks 2.27 is by Ivan Molodetskikh (YaLTeR), with a BSD-3-Clause notice
in `mod-port-licenses/MouseTweaks-BSD-3-Clause.txt`. The reference is the
[official release](https://modrinth.com/mod/mouse-tweaks/version/m8rwZENW).

Totem Tweaks 1.0.1 by RacialGamer is the behavioral reference for Atlas's
implementation and configuration screen. The reference license notice is
preserved in `mod-port-licenses/TotemTweaks-original-license.txt`.

## Inventory HUD + and Macro Keybinds

Atlas implements the observed behavior in Rust with Atlas settings widgets and vanilla item rendering.

| Behavioral reference | Original author and terms | SHA-256 |
| --- | --- | --- |
| `inventoryhud.fabric.1.21.4-3.4.27.jar` | DmitryLovin, All Rights Reserved unless otherwise explicitly stated | `747225370c37c3b18fe2a69817a3566a60ac3220b5b6e60da3cb723d3b96840a` |
| `1.21.4-macrokeybinds-fabric-1.3.1.jar` | Thomas7520, CC BY-NC-SA 4.0 | `437b31984da6d4d713055925495d038d9f1018b4bb955ae50aca0b95fe96e6a7` |

The reference JARs, code, and artwork are absent from the Atlas distribution.

## In-Game Account Switcher

The reference is `IAS-9.0.7+1.21.4-fabric.jar`, SHA-256
`6ef831ab457413ace30168d3304fbc609aa52cb68ce21b4417d611847e6ae2a5`.
VidTu and the original contributors retain their credits. IAS artwork is
provided under LGPL-3.0-or-later with `mod-port-licenses/IAS-LGPL-3.0.txt`.
The original source is [In-Game Account Switcher](https://github.com/The-Fireplace-Minecraft-Mods/In-Game-Account-Switcher).


## Gamma Utils artwork

The three unmodified status-effect PNGs under
`crates/atlas-client-render/src/atlas/assets/gammautils/` are from the supplied
Gamma Utils 2.2.6 JAR by Sjouwer, under GNU LGPL v3. Their original source paths
are `assets/gammautils/textures/mob_effect/{bright,dim,night_vision}.png`.
They remain separate artwork with their original license; they are not relicensed
as Atlas MIT artwork. Corresponding editable PNG source is included unchanged.
See `mod-port-licenses/LGPL-3.0-GammaUtils.txt`, `mod-port-licenses/GPL-3.0.txt`
and https://github.com/Sjouwer/gamma-utils for the original project.
