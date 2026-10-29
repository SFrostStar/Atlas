# atlas-server in a container (docs/server.md, section 3). Build context: the repository root:
#   docker build -f packaging/server/Dockerfile -t atlas-server .
#
# The image holds none of Minecraft's files. Prepare them on the host from your own Minecraft 26.3
# (atlas-prepare --prepare, see packaging/PLAY.txt) and mount the folder it writes, read-only:
#   docker run -d --name atlas -p 25565:25565 --stop-timeout 180 \
#     -v atlas-data:/data \
#     -v /path/to/.local/atlas-client/26.3:/opt/atlas/.local/atlas-client/26.3:ro \
#     -e ATLAS_EULA=true -e ATLAS_WHITELIST=true -e ATLAS_PLAYERS=Alex,Steve -e ATLAS_OP=Alex \
#     atlas-server
#
# ATLAS_EULA=true accepts the Minecraft EULA (https://aka.ms/MinecraftEULA) for this server. The first
# start writes /data/server.toml from ATLAS_* (EULA, WORLD, SEED, PORT, WHITELIST, PLAYERS, OP); later
# starts use the file. Manage it with `docker exec atlas atlas-server status|cmd|backup|logs ...`.
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
# The server build of the game (no sound, no controllers: it needs neither ALSA nor udev) as atlas-engine.
RUN cargo build --release --locked -p atlas-client --no-default-features --bin atlas-client \
    && cargo build --release --locked -p atlas-server --bin atlas-server \
    && mkdir /out && cp target/release/atlas-client /out/atlas-engine && cp target/release/atlas-server /out/

FROM debian:bookworm-slim
RUN useradd --system --home-dir /data --shell /usr/sbin/nologin atlas \
    && mkdir -p /data /opt/atlas/.local/atlas-client \
    && chown atlas:atlas /data
COPY --from=build /out/ /opt/atlas/
ENV PATH=/opt/atlas:$PATH \
    ATLAS_NO_AUDIO=1
WORKDIR /data
VOLUME /data
EXPOSE 25565
USER atlas
# SIGTERM makes atlas-server stop the game cleanly (it saves first): give it --stop-timeout 180.
CMD ["sh", "-c", "test -f server.toml || atlas-server init --yes || exit 1; exec atlas-server run"]
