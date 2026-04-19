# Running a World Compute WSS/443 Relay

**Target audience**: volunteers with a publicly-reachable machine (cloud VM, home server with port-forwarding, co-located hardware) who want to help donors behind strict firewalls join the mesh.

**Requirement**: a public IPv4 or IPv6 address and the ability to bind TCP port 443.

**Why this matters (spec 005 US1 / FR-007a)**: most institutional, corporate, and cloud firewalls permit only HTTPS on port 443. Donors on such networks cannot form a mesh connection to a regular libp2p relay. A WSS/443 relay listens on port 443 with TLS, indistinguishable from a regular HTTPS server, and bridges those donors into the global mesh.

## 1. Prerequisites

- Linux server (Ubuntu 24.04 LTS recommended; any systemd-based distro works).
- Public IPv4 and/or IPv6 with port 443 reachable (test: `curl https://<your-ip>` from another machine).
- 1 CPU core, 512 MB RAM, 5 GB disk — relays are lightweight; they forward bytes, not execute workloads.
- A TLS certificate for the hostname you operate from. Let's Encrypt via `certbot` works.
- The signed World Compute donor binary (see [quickstart.md](../../specs/005-production-readiness/quickstart.md)).

## 2. Generate or supply a TLS certificate

Option A — Let's Encrypt (recommended for public hostnames):

```bash
sudo apt-get install certbot
sudo certbot certonly --standalone -d relay.example.org
# Certificate is placed at /etc/letsencrypt/live/relay.example.org/
```

Option B — self-signed (not recommended; donors will refuse pin mismatch):

```bash
openssl req -x509 -newkey rsa:4096 -keyout relay.key -out relay.crt \
    -days 365 -nodes -subj "/CN=relay.example.org"
```

## 3. Start the relay

```bash
sudo worldcompute donor join --daemon \
    --wss-listen \
    --tls-cert /etc/letsencrypt/live/relay.example.org/fullchain.pem \
    --tls-key  /etc/letsencrypt/live/relay.example.org/privkey.pem
```

The `--wss-listen` flag enables the WSS/443 listener in addition to the normal
TCP 19999 / QUIC 19999 listeners. Binding to port 443 requires root (or
`setcap cap_net_bind_service=+ep`); either run with sudo or use a reverse
setcap on the binary.

## 4. Register with the mesh

The first time your relay connects to the bootstrap DHT, it announces itself
as a relay via the libp2p Identify protocol. Peers discover it via the Kademlia
DHT + peer-exchange. No client-side code update is needed — donors learn the
new relay's existence through gossip.

Expected log lines on a healthy relay:

```
[info] peer_id=12D3KooW... listening on /ip4/0.0.0.0/tcp/19999
[info] peer_id=12D3KooW... listening on /ip4/0.0.0.0/udp/19999/quic-v1
[info] peer_id=12D3KooW... listening on /ip4/0.0.0.0/tcp/443/tls/ws
[info] connected to bootstrap peer QmNnooDu7...
[info] relay mode active: accepting reservations
```

## 5. Monitor capacity

Relays have a reservation cap (default 128 simultaneous reservations). Once
full, new reservation requests are denied and donors try other relays.

```bash
worldcompute admin status --focus relays
```

Will report active reservation count, peak count, denied count.

## 6. Security posture

- **Traffic is Noise-encrypted end-to-end**; the relay cannot read payloads.
- **Relay operator liability**: you forward bytes; you do not execute workloads.
  You cannot be held responsible for the content of the traffic in any
  meaningful legal sense — you are a network router.
- **Rate limits**: the relay ships with sensible per-peer rate limits to
  prevent abuse. Tuning via `~/.worldcompute/config.toml` if needed.
- **Volunteer retirement**: project-operated launch relays (see
  `PUBLIC_LIBP2P_BOOTSTRAP_RELAYS` in `src/network/discovery.rs`) are
  retire-able without a client update once enough volunteer-run relays are
  online. Gossip + peer-exchange ensures clients discover new relays
  automatically.

## 7. Troubleshooting

| Symptom | Remedy |
|-|-|
| `Error: Address already in use (os error 98)` on port 443 | Another service (nginx, apache) is listening. Stop it or relocate the relay to a sub-path via reverse proxy. |
| `Error: Permission denied (os error 13)` on port 443 | Run with sudo, or `sudo setcap cap_net_bind_service=+ep /usr/local/bin/worldcompute`. |
| No reservations arriving | Your DNS / NAT isn't announcing correctly. Verify with `curl https://relay.example.org` from a third machine and check DHT routing with `worldcompute admin peers`. |

## 8. Uninstall

```bash
sudo systemctl stop worldcompute-relay
sudo systemctl disable worldcompute-relay
rm ~/.worldcompute -rf
```
