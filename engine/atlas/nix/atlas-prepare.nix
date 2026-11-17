# atlas-prepare: builds the game files Atlas reads from the player's own Minecraft 26.3 and the
# server jar they downloaded themselves. Nothing of Mojang's is fetched or packaged here.
#
# It works in a "root": research-lock.json (the pinned jars' hashes), the exporters' Java sources it
# compiles against those jars, and a writable .local/ for its scratch files. The store is read-only, so
# the package keeps that root in share/atlas/prepare and bin/atlas-prepare (atlas-prepare.sh)
# copies it somewhere writable unless --root is given. The exporters need a JDK of the pinned Java
# version (25): the default JAVA_HOME.
{
  lib,
  stdenv,
  rustPlatform,
  callPackage,
  coreutils,
  jdk25_headless,
  version,
}:
rustPlatform.buildRustPackage {
  pname = "atlas-prepare";
  inherit version;
  src = callPackage ./source.nix { };
  cargoLock.lockFile = ../Cargo.lock;

  cargoBuildFlags = [
    "--package=atlas-prepare"
    "--bin=atlas-prepare"
  ];

  doCheck = false;

  postInstall = ''
    root=$out/share/atlas/prepare
    install -Dm444 research-lock.json -t $root
    install -Dm444 crates/atlas-prepare/src/exporters.rs -t $root/crates/atlas-prepare/src
    mkdir -p $root/mods/atlas/src/test
    cp -r mods/atlas/src/test/java $root/mods/atlas/src/test/

    mkdir -p $out/libexec
    mv $out/bin/atlas-prepare $out/libexec/atlas-prepare
    substitute ${./atlas-prepare.sh} $out/bin/atlas-prepare \
      --subst-var out \
      --subst-var-by shell ${stdenv.shell} \
      --subst-var-by coreutils ${coreutils} \
      --subst-var-by java ${jdk25_headless}
    chmod +x $out/bin/atlas-prepare
  '';

  passthru.java = jdk25_headless;

  meta = {
    description = "Prepare Atlas's game files from your own copy of Minecraft: Java Edition 26.3";
    homepage = "https://playhorizon.dev";
    license = lib.licenses.mit;
    mainProgram = "atlas-prepare";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
