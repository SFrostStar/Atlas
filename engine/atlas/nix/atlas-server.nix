# atlas-server: server.toml, the CLI and the supervisor. It finds the game beside itself, so the
# engine is linked into the same bin/ (crates/atlas-server/src/engine.rs `program_in`).
{
  lib,
  rustPlatform,
  pkg-config,
  callPackage,
  atlas-engine,
  version,
}:
rustPlatform.buildRustPackage {
  pname = "atlas-server";
  inherit version;
  src = callPackage ./source.nix { };
  cargoLock.lockFile = ../Cargo.lock;

  cargoBuildFlags = [
    "--package=atlas-server"
    "--bin=atlas-server"
  ];

  nativeBuildInputs = [ pkg-config ];

  doCheck = false;

  postInstall = ''
    ln -s ${lib.getExe atlas-engine} $out/bin/atlas-engine
  '';

  passthru.engine = atlas-engine;

  meta = {
    description = "Host a Minecraft: Java Edition 26.3 server with Atlas, the game rewritten in Rust";
    longDescription = ''
      Holds none of Mojang's files: the server reads the game files you prepare from your own copy of
      Minecraft 26.3 with atlas-prepare (docs/hosting.md).
    '';
    homepage = "https://playhorizon.dev";
    license = lib.licenses.mit;
    mainProgram = "atlas-server";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
