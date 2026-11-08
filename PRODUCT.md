# Atlas

<!-- impeccable:product-schema 1 -->

## Platform
web

## Product and stack
Atlas is SFrostStar's independent Minecraft client based on Horizon. The desktop launcher is Electron, JavaScript, HTML and CSS; the native game uses Rust and wgpu. The website is a separate static surface under `website/`.

## Users and jobs
Players create and switch instances, manage compatible native mods and resource packs, choose accounts and launch the game. Developers can open their local source and runtime. Existing worlds, account data and settings must survive UI changes.

## Confirmed direction
The latest approved scope is stage 1 (faithful built-in Mod Menu) and stage 2 (Electron launcher redesign). Stages 3–4 of broader game UI work are deferred. The launcher reference calls for the existing landscape across the window, a compact top navigation instead of a sidebar, translucent frosted-glass panels with real blur and restrained rounded controls. Keep Russian and English, dark and light themes, custom accents and personalisation. Mod Menu faithfully matches Mod Menu 21.0.0 rather than adopting the launcher's glass styling.

## Constraints
Keep the launcher on Electron and game UI on its Rust renderer. Preserve all working flows, source attributions and licences. Do not publish, commit or release as a side effect of implementation. All live testing uses isolated scratch profiles.

## Brand
Atlas with capital A and the existing orange A logo. Geist is the game's existing smooth UI font and is shared by the launcher. Keep concise functional Russian/English copy; no decorative slogans beyond the existing optional home headline.
