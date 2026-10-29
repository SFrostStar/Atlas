# Built from the repository root: docker build -f crates/atlas-mod-registry/Dockerfile .
FROM rust:1-slim AS build
WORKDIR /src
# The root manifest only for the shared crate's workspace settings (edition, lints).
COPY Cargo.toml Cargo.toml
COPY crates/atlas-mods crates/atlas-mods
# atlas-mods' optional dependency: cargo needs its manifest even with the feature off.
COPY crates/atlas-shader-api crates/atlas-shader-api
COPY crates/atlas-mod-registry crates/atlas-mod-registry
RUN cargo build --release --locked --manifest-path crates/atlas-mod-registry/Cargo.toml

FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/crates/atlas-mod-registry/target/release/atlas-mod-registry /usr/local/bin/
ENV REGISTRY_DB=/data/registry.sqlite3
# Railway mounts the ratings' volume at /data owned by root: hand it to nobody, then run as nobody (setpriv is
# util-linux's, in every Debian image).
CMD ["sh", "-c", "mkdir -p /data && chown nobody:nogroup /data && exec setpriv --reuid=nobody --regid=nogroup --clear-groups atlas-mod-registry"]
