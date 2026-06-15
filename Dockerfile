# Copyright 2026 Query Farm LLC - https://query.farm
#
# Container for the easter VGI worker's HTTP transport. DuckDB clients ATTACH it
# over http:// — see README.
#
# The vgi Rust SDK's `--http` mode binds an ephemeral port on 127.0.0.1 and
# announces it as `PORT:<n>` on stdout. We bridge external :8080 traffic to that
# announced port with socat so Fly (and any other proxy) can route to it.

# ---- build stage -----------------------------------------------------------
FROM rust:1.86-slim-bookworm AS build
WORKDIR /app
# Cache dependency compilation: copy manifests, build a stub, then the sources.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release --locked && rm -rf src
COPY src ./src
# Bust the stub's cached build artifact, then build for real.
RUN touch src/main.rs && cargo build --release --locked

# ---- runtime stage ---------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends socat ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/vgi-easter /usr/local/bin/vgi-easter
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Fly routes external traffic to this internal port (see fly.toml).
EXPOSE 8080
CMD ["/usr/local/bin/docker-entrypoint.sh"]
