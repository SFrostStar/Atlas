# The game (atlas-client, with sound, controllers and Atlas's extras) and atlas-ctl, the tool
# that drives it (docs/agent-control.md). Start it with `atlas-client --resources DIR`, DIR being
# what atlas-prepare wrote.
{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  callPackage,
  alsa-lib,
  udev,
  vulkan-loader,
  wayland,
  libxkbcommon,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
  libxcb,
  version,
}:
let
  # wgpu and winit open these with dlopen, so the linker never records them.
  dlopened = [
    vulkan-loader
    wayland
    libxkbcommon
    libx11
    libxcursor
    libxi
    libxrandr
    libxcb
  ];
in
rustPlatform.buildRustPackage {
  pname = "atlas-client";
  inherit version;
  src = callPackage ./source.nix { };
  cargoLock.lockFile = ../Cargo.lock;

  cargoBuildFlags = [
    "--package=atlas-client"
    "--bin=atlas-client"
    "--package=atlas-client-lib"
    "--bin=atlas-ctl"
  ];

  nativeBuildInputs = [ pkg-config ];
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [
    alsa-lib
    udev
  ];

  doCheck = false;

  postFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    patchelf --add-rpath ${lib.makeLibraryPath dlopened} $out/bin/atlas-client
  '';

  meta = {
    description = "Atlas, Minecraft: Java Edition 26.3 rewritten in Rust, and atlas-ctl";
    homepage = "https://playhorizon.dev";
    license = lib.licenses.mit;
    mainProgram = "atlas-client";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
