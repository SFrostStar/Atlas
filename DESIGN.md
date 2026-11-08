# Atlas application UI

The launcher follows the user's newer screenshot direction: a full-window Atlas sunset landscape, floating top pill navigation and translucent frosted-glass controls. Mod Menu preserves its Java reference, and the public website has a separate design record. Broader in-game settings redesign is deferred; the neutral palette below remains useful for that future stage but does not authorize changing game settings now.

## Shared tokens

| Role | Dark | Light |
| --- | --- | --- |
| Canvas | `#151719` | `#f2f4f6` |
| Panel | `#1e2125` | `#ffffff` |
| Raised / hover | `#272c32` | `#e9edf1` |
| Text | `#f4f3ef` | `#20252c` |
| Secondary text | `#a7adb5` | `#5d6774` |
| Dividers | `#353a42` | `#d4dbe3` |
| Default accent | `#ff8a3d` | `#ff8a3d` |

Primary action text is `#211309`. Accent hover is `#ffa363`. Existing selectable accents override the accent roles, not the Atlas logo. Keep semantic success and error colours independent from the selected accent.

## Typography and geometry

Self-host Geist Regular (400), Medium (500) and SemiBold (600), copied from the engine's existing licensed font assets, with Segoe UI fallback. Body 15–16px, compact metadata 12–13px, page headings 26px. Never reduce functional text below 11px.

The launcher has no global sidebar. Its centered pill navigation includes Settings. A symmetric grid reserves both brand and account columns, independent of account name and support visibility. Support is a dismissible fixed chip in the lower-right corner and returns on a new launcher session. The launch action and compact instance switcher sit together at the lower left of the full-window scene. Home has no management tabs. The Instances workspace places compact selectable cards above the selected instance's two tabs: Instance files and Mods. Files opens saves, atlas-mods, resourcepacks, shaderpacks, config and logs; Mods retains per-instance installation/search controls. Selecting a card stays in the workspace, and existing edit/rename/delete context menus remain available. Source controls live in Workshop and Settings.

Use real `backdrop-filter: blur(22–30px)` and translucent graphite or light glass, rather than opaque panels pretending to blur. Glass is the explicit user brief. Keep text contrast with a sufficiently opaque tint. Primary navigation and the launch action may be pills; form controls, dialogs and settings panels use modest 8–16px radii. Accent choices remain independent of the fixed orange Atlas logo. Retain a solid-surface fallback where backdrop-filter is unsupported.

## Behaviour and review

The one-second boot stays unchanged. Route changes use a 280ms fade and 10px rise; primary tabs crossfade their own accent outlines and soft tint over180ms with a restrained scale reveal over260ms. No travelling indicator or spring motion. Focused pages add a 300ms scene scrim, and dialogs use a 220ms entrance/exit. State refreshes do not restart route motion. Per-instance RAM is a keyboard-accessible 1–32GB slider, default4GB, with immediate value feedback and persistence on change. Keep the existing one-second boot and reduce motion preferences. Hover/focus transitions are 160ms. Every action remains keyboard reachable with a visible 2px accent outline. Existing context menus, instance icon/colour controls, install progress and all preference flows remain functional.

Visual verification uses fresh scratch Electron profiles in Russian/English, light/dark themes at 1360px and 1050px. All routes, fonts/images, content overflow, catalogue search/empty states, category links, RAM, instance icons/colours, context menus and saved language/theme/accent have been exercised. The newer user scope permits one active subagent; design review stays within the assigned work, with no additional delegation.


## Game settings

SFrostStar chose vanilla Minecraft styling for Atlas Settings and its subpages. Use the standard VideoSettingsScreen layout, textured rectangular Minecraft buttons and sliders, neutral list surfaces and standard On/Off controls. Keep Atlas's smooth Geist font, including Cyrillic, as explicitly requested. The sidebar graphics browser is no longer a player-facing entry. Keep the launcher glass theme independent. Preserve setting values, navigation and rendering capabilities.
