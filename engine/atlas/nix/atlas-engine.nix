# The server build of the game (`atlas-client --no-default-features`): the game without sound,
# controllers or Atlas's online extras, so it links neither ALSA nor udev. atlas-server runs it
# headless as `atlas-engine` (docs/hosting.md, packaging/server/Dockerfile).
{
  lib,
  rustPlatform,
  pkg-config,
  callPackage,
  version,
}:
rustPlatform.buildRustPackage {
  pname = "atlas-engine";
  inherit version;
  src = callPackage ./source.nix { };
  cargoLock.lockFile = ../Cargo.lock;

  cargoBuildFlags = [
    "--package=atlas-client"
    "--bin=atlas-client"
  ];
  buildNoDefaultFeatures = true;

  nativeBuildInputs = [ pkg-config ];

  # The game's tests read the prepared vanilla files in .local/, which a build never has.
  doCheck = false;

  postInstall = ''
    mv $out/bin/atlas-client $out/bin/atlas-engine
  '';

  meta = {
    description = "Atlas's headless game engine, run by atlas-server";
    homepage = "https://playhorizon.dev";
    license = lib.licenses.mit;
    mainProgram = "atlas-engine";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
