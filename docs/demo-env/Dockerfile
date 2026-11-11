# Environment for recording docs/demo.gif with VHS. See docs/demo.md.
#
#   docker build -f docs/demo-env/Dockerfile -t panosync-demo .
#   docker run --rm -v "$PWD/docs:/vhs" panosync-demo
#
# Two virtual X displays stand in for two computers, so the demo never touches your own clipboard.

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p panosync -p panosync-relay

FROM ghcr.io/charmbracelet/vhs:v0.12.1
# Timeouts and retries: a stalled mirror connection otherwise hangs the build forever.
RUN printf 'Acquire::ForceIPv4 "true";\nAcquire::http::Timeout "20";\nAcquire::Retries "5";\n' > /etc/apt/apt.conf.d/99demo \
    && apt-get update \
    && apt-get install -y --no-install-recommends tmux xvfb xclip \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/panosync /src/target/release/panosync-relay /usr/local/bin/
COPY docs/demo-env/demo.bashrc /etc/panosync-demo.bashrc
COPY docs/demo-env/tmux.conf /etc/panosync-demo.tmux.conf
COPY docs/demo-env/start-demo.sh /usr/local/bin/start-demo
COPY docs/demo-env/render.sh /usr/local/bin/render-demo
RUN chmod 755 /usr/local/bin/start-demo /usr/local/bin/render-demo
ENTRYPOINT ["/usr/local/bin/render-demo"]
