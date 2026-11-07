# Atlas website

A small bilingual static landing page. Publish only `public/`, never the workspace root. GitHub Pages deployment is defined in `../.github/workflows/pages.yml` and runs when the public site changes on `main`.

For local preview, run `python -m http.server 4173 --directory website/public` from the repository root. No Node dependencies, build step, cookies or analytics. The only external request is the public GitHub latest-release API. If it fails, the verified Atlas 1.0.0 Windows portable download remains usable. Future versioned Windows ZIPs are picked up automatically; Update ZIPs are excluded.

## Asset provenance

- `public/assets/icon.png`: existing Atlas logo, copied unchanged from `assets/icon.png`.
- `public/assets/night.png`: existing Atlas night artwork, copied unchanged from `engine/atlas/crates/atlas-client-render/src/atlas/assets/night.png`.
- `public/assets/landscape.png`: existing Atlas landscape, copied unchanged from `assets/atlas-landscape.png`.
- Manrope fonts: Google Fonts distribution, self-hosted under the included SIL Open Font License (`public/assets/OFL-Manrope.txt`).
- GitHub icon: GitHub mark, used only to identify the source-code link. Other interface icons are authored simple SVG geometry.

Player data, screenshots, game resources, application builds and downloaded mods are not part of this website.
