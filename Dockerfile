# Builds only the relay server (panosync-relay). Used by Coolify and for the ghcr.io image.
# Builder and runtime are both Debian bookworm, so the binary's glibc matches the runtime.
FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release --locked -p panosync-relay

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 panosync
COPY --from=build /app/target/release/panosync-relay /usr/local/bin/panosync-relay
USER panosync
ENV PORT=8080
EXPOSE 8080
CMD ["panosync-relay"]
