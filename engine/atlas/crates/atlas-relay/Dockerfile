# The Play with friends relay, for anyone to run (docs/play-with-friends.md, "Running your own relay").
# Build context: the repository root (the workspace's manifests and lockfile):
#   docker build -f crates/atlas-relay/Dockerfile -t atlas-relay .
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p atlas-relay \
    && cp target/release/atlas-relay /atlas-relay

FROM debian:bookworm-slim
COPY --from=build /atlas-relay /usr/local/bin/atlas-relay
ENV PORT=7000
EXPOSE 7000
USER nobody
CMD ["atlas-relay"]
