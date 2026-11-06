#!/bin/sh
# Called inside rust:alpine3.22 by game-build.yml, without release credentials.
set -eu

apk add --no-cache build-base pkgconf alsa-lib-dev eudev-dev python3 git
git config --global --add safe.directory /work
# Match the stable channel used by the other release jobs, not the image's bundled Rust.
rustup update stable --no-self-update
export RUSTUP_TOOLCHAIN=stable
rustup component add llvm-tools

# Static musl cannot dlopen the graphics/window libraries. Keep audio and controllers,
# and link against Alpine's native libraries rather than Ubuntu's glibc libraries.
export RUSTFLAGS='-C target-feature=-crt-static'
target=x86_64-unknown-linux-musl
bin="target/$target/release"
cargo build --release --locked --target "$target" \
  -p atlas-client -p atlas-client-lib --bin atlas-client --bin atlas-ctl \
  -p atlas-prepare --bin atlas-prepare -p atlas-server --bin atlas-server

set -- "$bin/atlas-client" "$bin/atlas-ctl" "$bin/atlas-prepare" "$bin/atlas-server"
python3 tools/check_imports.py --musl "$@"
python3 tools/check_cpu_baseline.py "$@"

# Even --help must never touch real player data.
export ATLAS_HOME="$(mktemp -d)"
export ATLAS_LAUNCHER_HOME="$(mktemp -d)"
export XDG_DATA_HOME="$(mktemp -d)"
export XDG_CONFIG_HOME="$(mktemp -d)"
export ATLAS_LAUNCHER_NO_SELF_UPDATE=1 ATLAS_NO_AUDIO=1
"$bin/atlas-client" --help > "$ATLAS_HOME/client-help"
grep -q -- '--resources' "$ATLAS_HOME/client-help"
"$bin/atlas-ctl" --help > /dev/null
"$bin/atlas-prepare" --help > "$ATLAS_HOME/prepare-help"
grep -q -- '--minecraft-dir' "$ATLAS_HOME/prepare-help"
for binary in atlas-prepare atlas-server; do
  test "$("$bin/$binary" --version)" = "$binary $ATLAS_VERSION"
done

cargo run --locked --target "$target" -p xtask -- package \
  "$bin/atlas-client" dist/atlas-linux-musl-x86_64
tar -C dist -czf dist/atlas-linux-musl-x86_64.tar.gz atlas-linux-musl-x86_64
