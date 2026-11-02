# Hearth server image (multi-stage).
FROM rust:1.99-slim AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY modules ./modules
RUN cargo build --release -p hearth-server

FROM node:22-slim AS ui
WORKDIR /ui
COPY web/package.json web/package-lock.json* ./
RUN npm install --no-audit --no-fund
COPY web ./
RUN npm run build

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=rust /src/target/release/hearth-server /bin/hearth-server
COPY --from=ui /ui/build /srv/ui
ENV PORT=8080 BIND_ADDR=0.0.0.0
EXPOSE 8080
ENTRYPOINT ["/bin/hearth-server"]
