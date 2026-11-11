# Self-hosting the relay

`panosync-relay` is a single binary with no state and no config file. It listens with plain HTTP and
WebSocket on `0.0.0.0:$PORT` (default `8080`) and serves two paths:

- `GET /ws?channel=<64 hex chars>`: the WebSocket endpoint clients connect to;
- `GET /health`: returns `ok`.

Clients should reach it through **TLS** (`wss://`). The relay does not terminate TLS itself, so put
it behind a reverse proxy that does: Coolify, Caddy, nginx, Traefik and others all work. The proxy
must pass WebSocket upgrades through. Without TLS everything is still end-to-end encrypted, but the
connection metadata (and the channel id in the URL) travel in the clear.

Pick one option:

| Option | Good for | TLS |
|---|---|---|
| [Coolify](#coolify) | you already run Coolify | automatic |
| [Docker](#docker) | any machine with Docker | add a proxy |
| [Binary + systemd](#binary-with-systemd) | a small VPS without Docker | add a proxy |
| [Caddy](#https-with-caddy) or [nginx](#https-with-nginx) in front | TLS for the two options above | yes |
| [Local network only](#local-network-only) | devices that are always on the same LAN | none |

After setting it up, check it with `curl https://<your domain>/health` (should print `ok`), then
point the clients at it with `panosync setup --server wss://<your domain>`.

> **How these instructions were tested** is noted at the end of each section. Everything marked
> *tested* was run end to end: two simulated devices exchanging messages through that setup, using the
> project's end-to-end test (`PANOSYNC_TEST_RELAY=<url> cargo test --test end_to_end`), which you can
> also use to check your own deployment.

## Coolify

1. **+ New → Resource → Public Repository** (or *Private Repository (GitHub App)* for a fork you keep
   private) and enter this repository's URL, branch `main`.
2. **Build Pack:** `Dockerfile`. **Ports Exposes:** `8080`.
3. **Domains:** `https://panosync.example.com` (point a DNS `A` record at the server first). Coolify gets a
   Let's Encrypt certificate and handles WebSocket upgrades through its proxy without extra settings.
4. **Deploy**, then open `https://panosync.example.com/health`.

No domain? Coolify's `sslip.io` addresses work too (e.g. `https://panosync.1.2.3.4.sslip.io`).
With *Auto Deploy* on, every push to the tracked branch redeploys the relay; connected clients
reconnect on their own within a few seconds.

*Tested:* the project's own relay runs this way. After deploys, `/health` answered `ok` and the
end-to-end test passed against it over `wss://`. A redeploy after renaming the GitHub repository also
worked without changing anything in Coolify.

## Docker

```bash
docker run -d --name panosync-relay --restart unless-stopped \
  -p 127.0.0.1:8080:8080 ghcr.io/msbayindir/panosync-relay:latest
```

`127.0.0.1:` keeps the port reachable only from the same machine, for a reverse proxy running there.
Use `-p 8080:8080` if the proxy runs elsewhere, or for [local network only](#local-network-only) use.
Images are published for `linux/amd64` and `linux/arm64`. To build it yourself: `docker build -t panosync-relay .`
in a checkout of this repository.

With Docker Compose and Caddy together:

```yaml
services:
  relay:
    image: ghcr.io/msbayindir/panosync-relay:latest
    restart: unless-stopped
  caddy:
    image: caddy:2
    restart: unless-stopped
    ports: ["80:80", "443:443"]
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
volumes:
  caddy_data:
```

with a `Caddyfile` of `panosync.example.com { reverse_proxy relay:8080 }`.

*Tested:* the image built from this repository's `Dockerfile` (`/health` → `ok`, end-to-end test passed
against the container). The `ghcr.io` image is produced by the release workflow from the same Dockerfile.
The Compose file above was not run as is; its parts (relay container and Caddy on one Docker network)
were tested as described under Caddy.

## Binary with systemd

1. Build the relay (`cargo build --release -p panosync-relay`, the binary is
   `target/release/panosync-relay`) on the server or for its architecture, and install it:
   ```bash
   sudo install -m 755 panosync-relay /usr/local/bin/panosync-relay
   ```
2. Install the service from [packaging/panosync-relay.service](../packaging/panosync-relay.service):
   ```bash
   sudo cp packaging/panosync-relay.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now panosync-relay
   curl http://127.0.0.1:8080/health
   ```
3. Follow the logs with `journalctl -u panosync-relay -f`. Change the port with
   `sudo systemctl edit panosync-relay` and `[Service]` / `Environment=PORT=9000`.

The unit runs the relay as a throwaway user (`DynamicUser=yes`) with a read-only file system and no
capabilities; it needs nothing else. The relay always listens on all interfaces, so if the proxy runs
on the same machine, block the port from the outside with your firewall.

Prebuilt release archives contain only the client; the relay is distributed as the Docker image or built
from source.

*Tested:* the unit above under systemd 252 (Debian 12) in a container: `systemd-analyze verify` reported no
problems, the service started as a dynamic user, `/health` answered `ok` and the end-to-end test passed
against it. The relay binary came from the Docker image (glibc, bookworm).

## HTTPS with Caddy

Caddy gets and renews certificates automatically and proxies WebSockets without extra settings.
`/etc/caddy/Caddyfile`:

```
panosync.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

Then `sudo systemctl reload caddy`. Ports 80 and 443 must be reachable from the internet for the
certificate challenge.

*Tested:* Caddy 2 in front of the relay container, with the end-to-end test passing through it. The
test used a plain `:80` site block instead of a domain, because no public domain was available for a
certificate; the automatic HTTPS part is standard Caddy behaviour and was not exercised here.

## HTTPS with nginx

nginx does not pass WebSocket upgrades on by default; the `Upgrade` and `Connection` headers below are
required. Without them `/health` still works but clients cannot connect.

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

server {
    listen 443 ssl;
    http2 on;
    server_name panosync.example.com;

    ssl_certificate     /etc/letsencrypt/live/panosync.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/panosync.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        # Clients ping every 25 seconds; keep idle connections open longer than that.
        proxy_read_timeout 120s;
        proxy_send_timeout 120s;
    }
}

server {
    listen 80;
    server_name panosync.example.com;
    return 301 https://$host$request_uri;
}
```

Get the certificate with certbot (`sudo certbot certonly --nginx -d panosync.example.com`) or your
usual tool, then `sudo nginx -t && sudo systemctl reload nginx`.

*Tested:* the `location` block above (with `listen 80` instead of TLS) in front of the relay container:
the end-to-end test passed through it. With a plain `proxy_pass` and no `Upgrade`/`Connection` headers the
same test failed, as expected. The TLS lines are standard nginx configuration and were not tested here
(no public domain or certificate was available); `nginx -t` accepted the full file above with a
self-signed certificate in its place (nginx 1.30).

## Local network only

If your devices are always on the same network (home, office) you can skip the internet-facing part:
run the relay on any machine of that network and connect with `ws://`:

```bash
docker run -d --name panosync-relay --restart unless-stopped -p 8080:8080 ghcr.io/msbayindir/panosync-relay:latest
# on each device:
panosync setup --server ws://192.168.1.10:8080
```

`panosync setup` warns that `ws://` does not use TLS. Clipboard contents are still end-to-end
encrypted, but anyone on that network can see the channel id, when you copy something and roughly how
much. Give the relay machine a fixed IP address (a DHCP reservation in your router), and make sure the
port is not forwarded to the internet.

*Tested:* the end-to-end test over `ws://` against a relay on the same machine, which is the same code path
as a relay elsewhere on the LAN.

## Operating notes

- **Logs** contain connection events with the first 8 characters of the channel id, and never message
  contents (which the relay cannot read anyway).
- **Limits** are fixed: 1 MiB per message, 16 devices per channel, 50 messages per 10 seconds per
  connection. The relay keeps everything in memory and needs very little of it.
- **Updating** is replacing the binary or the image and restarting; clients reconnect with backoff
  (1 s up to 30 s). Relay and clients only need to share the protocol version (see
  [architecture.md](architecture.md)).
