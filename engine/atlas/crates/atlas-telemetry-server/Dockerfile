# Built from the repository root: docker build -f crates/atlas-telemetry-server/Dockerfile .
FROM rust:1-slim AS build
WORKDIR /src
# The root manifest only for the shared crate's workspace settings (edition, lints).
COPY Cargo.toml Cargo.toml
COPY crates/atlas-telemetry crates/atlas-telemetry
COPY crates/atlas-telemetry-server crates/atlas-telemetry-server
RUN cargo build --release --locked --manifest-path crates/atlas-telemetry-server/Cargo.toml

FROM debian:stable-slim
COPY --from=build /src/crates/atlas-telemetry-server/target/release/atlas-telemetry-server /usr/local/bin/
ENV ATLAS_TELEMETRY_DB=/data/telemetry.sqlite3 ATLAS_TELEMETRY_TRUST_PROXY=1
CMD ["atlas-telemetry-server"]
