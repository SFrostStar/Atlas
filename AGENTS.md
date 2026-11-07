# Atlas workspace

Atlas is maintained by SFrostStar. The launcher is Electron; the native Rust game
source is in `engine/atlas`. Read that directory's AGENTS.md before engine edits.

## Git and releases

- Create commits, push branches, create tags or publish releases only when
  author explicitly requests that action. Completing an implementation or
  passing tests does not authorize publication.
- The initial private-repository import is authorized by the repository-creation
  request. This exception does not authorize subsequent automatic commits/pushes.
- Keep the GitHub repository private unless SFrostStar explicitly asks otherwise.

## User data

- Never stage/upload player worlds, accounts, tokens, control sockets, logs,
  screenshots, prepared/downloaded Minecraft resources, local tools or builds.
- Preserve `.gitignore` protections. Do not bypass them with `git add -f`.
- Before an authorized commit/push, inspect the staged file list and run
  `python scripts/check_repository_privacy.py`.
- Run gameplay tests only with fresh scratch profiles. Preserve original
  copyright notices and dependency licenses.

## Mod ports

- SFrostStar requires faithful Java-to-Rust ports: preserve the supplied mod
  version's behavior, calculations, HUD layout, textures, colors, animation timing,
  tooltips and settings. A similarly named simplified feature is not a port.
- Inspect the supplied mod and compare concrete cases against it. Discuss any
  necessary differences before substituting behavior, and report unimplemented
  features honestly. AppleSkin 3.0.10 for Minecraft 26.3 is the current reference.
- Commit descriptions must contain only the text SFrostStar requests; do not add
  an agent attribution trailer against that preference.
