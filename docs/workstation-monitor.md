# Workstation monitor (Baylee Pulse)

`baylee-monitor` is an independent, read-only Linux workstation service. It loads
neither the game engine nor a CUDA context. The responsive German dashboard uses
embedded HTML/CSS/JavaScript: no frontend build, CDN, database or external assets.

## Run

```sh
nice -n 10 cargo build --locked --profile tools -p baylee-monitor -j 2
target/tools/baylee-monitor --bind 127.0.0.1:8787
```

Open `http://127.0.0.1:8787`. For Android on the same trusted LAN, use the machine's
explicit LAN address in `--bind` and the URL. Wildcards are rejected. Everyone who
can reach the address can read telemetry; there is no login. Do not publish this
port to the Internet. The service changes no firewall/router configuration.

It cannot kill, pause or change processes. It reads names and PIDs, never argv,
environments or usernames. The footer offers the exact embedded source and build
manifests, even before publication.

| CLI option | Default | Meaning |
| --- | --- | --- |
| `--bind` | `127.0.0.1:8787` | Explicit local/LAN socket address |
| `--allow-host` | none | Another exact `HOST:PORT` browsers may use (repeatable) |
| `--interval-ms` | `1000` | Sampling interval, 500–10000 ms |
| `--history-minutes` | `60` | RAM history, 1–120 minutes; UI shows up to one hour |

### Opening it from a phone

On the phone, type the address exactly as Pulse prints it at startup, including
`http://`: on this workstation `http://192.168.0.103:8787`. Pulse speaks plain
HTTP on a trusted LAN; `https://192.168.0.103:8787` fails with a browser
connection or SSL error, because there is no TLS on the port. If the browser
offers an "always use secure connections" upgrade, the explicit `http://` URL
is the one to keep as a bookmark.

Every request must name an accepted authority in its `Host` header (which stops
DNS rebinding), and a browser's `Origin`, when sent, must be that same authority
over `http` (which stops other sites reading telemetry). `Sec-Fetch-Site:
cross-site` is refused. Accepted are the bind address and each `--allow-host`
value, for a phone that opens Pulse by a name such as `bigace.local:8787` or
`bigace.fritz.box:8787`. Names are exact and case-insensitive, with a port and
without wildcards; an IPv6 literal is written `[addr]:port`. A page loaded under
one accepted name cannot read through another. At startup Pulse prints every
accepted URL to the journal.

A refused request gets a short plain-text 403 in German that names the address
the browser used and lists every accepted URL, with the reminder to use
`http://`, not `https://`. The first 16 distinct refused `Host` values are also
logged, so `journalctl --user -u baylee-monitor` shows what a phone sent.

No service-specific environment variables. The optional user unit is
`scripts/systemd/baylee-monitor.service`. Install the binary in
`~/.local/lib/baylee-monitor/` and the unit in `~/.config/systemd/user/`, adjust its
explicit LAN address if desired, then run `systemctl --user daemon-reload` and
`systemctl --user enable --now baylee-monitor`. Nice 10, low CPU/IO scheduling
weight, 192 MiB memory limit, restart on failure. It requires a running user
manager; installation does not enable lingering or change root permissions.
After updates, reinstall the binary and restart the unit. Stop with
`systemctl --user stop baylee-monitor`.

### Starting before the network

A user unit cannot wait for the system's `network-online.target`, and at login
the LAN address may not be assigned yet: until DHCP has finished, binding it
fails with `EADDRNOTAVAIL` ("Cannot assign requested address"). Pulse then logs
one line and retries the bind with a backoff from 250 ms up to 2 s, so it
listens within about two seconds of the address appearing. After ten minutes
without the address it exits with an error and the unit's `Restart=on-failure`
starts it again five seconds later. Every other bind error (port in use, an
address of another machine) fails at once.

On 2 November 2026 the journal seemed to show an hour without Pulse after
boot. The boot's monotonic timestamps tell otherwise: DHCP assigned
192.168.0.103 at 11.9 s, the link dropped and came back, the lease returned at
22.6 s, chronyd stepped the wall clock by +3600.9 s at 25.8 s, and Pulse
listened at 28.6 s, on its fourth start. The hour was the clock step.

## Local power-button setting (24 October 2026)

At the owner's request, the training workstation ignores power-button events.
This is an independent host setting, not a capability of the read-only monitor.
`/etc/systemd/logind.conf.d/90-ignore-power-button.conf` contains:

```ini
[Login]
HandlePowerKey=ignore
HandlePowerKeyLongPress=ignore
```

`sudo systemctl reload systemd-logind` applied the setting without restarting
logind or its sessions. Both live D-Bus properties were verified as `ignore`.
The file persists across reboots. Removing this drop-in and reloading logind
restores the remaining configuration's defaults. A firmware-enforced hard
power-off from holding the button cannot be prevented by this OS setting.
No graphical power manager or other power-button inhibitor was active when
verified; a future desktop environment that takes over the key needs its own
matching setting.

## Metric semantics

With sysinfo 0.40, unavailable uptime/load values are JSON `null` and displayed
as unavailable, rather than a fabricated zero. Failure to initialize a required
system provider returns a startup error. Interface names are used in their
native OS representation for physical-device detection and converted only for
display; collecting aggregate rates does not allocate an extra interface list.

- CPU overview is a percentage of all logical CPUs; process CPU uses 100% per
  logical CPU and can exceed 100%. Physical cores and threads are separate.
  Startup waits one sampling interval for a second CPU observation.
- System RAM uses OS available-memory accounting. Process RAM is RSS, including
  shared pages, so its sum is not total RAM use. Disk I/O is bytes per elapsed
  monotonic second where `/proc` allows reading it. Restricted processes may
  return zero CPU/RSS/I/O from the unprivileged OS provider.
- GPU: dynamic NVIDIA NVML reads utilization, VRAM, board watts, power limit,
  fan, temperature and compute/graphics processes without spawning `nvidia-smi`.
  Duplicate PIDs across the two lists count once per device. All GPUs appear in
  hardware details; overview GPU/VRAM charts use device 0. Power sums available
  readings across all GPUs. AMD GPU/NPU telemetry is not implemented yet.
- Per-process GPU utilization is a recent driver sample, not a share allocated
  from global utilization. Missing/unsupported samples are null/`—`, never a
  guessed zero or stale busy value. Percentages need not sum to device load.
- CPU power comes from readable RAPL package energy differences, with counter
  wrap handling. Core subdomains are excluded to avoid double counting. An
  unreadable package makes the total unavailable. No permissions are changed.
  On this 3900X, `energy_uj` exists but is not readable by the service user.
- Whole-PC and per-process watts are not measurable here. GPU watts are not wall
  power; there is no TDP, kWh or cost estimate. Invalid temperature readings below
  −20°C or above 150°C are discarded.
- Network plots sum physical interfaces, excluding loopback/bridges/veth to avoid
  double counting; virtual interface names remain visible. Filesystem capacity
  refreshes every 15 seconds; removable disks are omitted.

## Transport and cost

One blocking sampler supplies all clients at 1 Hz. No per-thread process scan or
parallel scan workers. HTTP uses one async thread. Full current process lists are
serialized once per tick; only compact aggregate points enter the bounded history.
History is in RAM and resets on restart. No telemetry files are written.

`GET /api/snapshot` returns `{"type":"sample","snapshot":…}`, or 503 at startup.
`GET /ws` sends `{"type":"hello","facts":…,"history":…}`, then sample messages.
The UI deduplicates timestamps overlapping between hello and the first sample.
Timestamps are Unix milliseconds; rate calculations use monotonic time.

At most 12 concurrent WebSocket viewers; extra upgrades receive 429. Latest-value
channels replace old snapshots, never queue them. Slow writes time out after five
seconds. Incoming application messages close the socket; there are no commands.
Browser reconnect uses backoff, stale measurements are marked, background tabs
stop drawing, and gaps in chart history remain gaps. Pointer inspection shows
values and timestamps.

Checks: `cargo test -p baylee-monitor`,
`cargo clippy -p baylee-monitor --all-targets -- -D warnings`,
`node --check crates/baylee-monitor/web/app.js`. Tests cover history/live updates,
connection limits, command rejection, permit release, cross-origin/rebinding
rejection, `--allow-host` parsing and admission, the plain-text refusal over HTTP
and WebSocket, waiting for an unassigned bind address and energy-counter wrap.
Browser checks cover desktop/mobile layout,
filter/sort, reconnection and missing sensors; close test browsers afterward.

## Initial workstation verification (23 October 2026)

Installed as the `ace` user service, enabled for user-manager startup, at
`http://192.168.0.103:8787`. The user has `Linger=no`: this is not a system boot
service independent of login. No root/user-linger settings were changed. The
binary lives in `~/.local/lib/baylee-monitor/baylee-monitor`; the installed unit
uses the LAN address, while the repository template defaults to loopback.

During the 54.28M-parameter GPU training continuation, a 15-second measurement
with one snapshot reader measured 1.86% of **one** logical CPU, about 22 MiB of
systemd-accounted service memory (roughly 28 MiB RSS including shared mappings),
and approximately 40 kB per snapshot. Sampling took 100 ms wall time on average
(170 ms maximum); much of that is sensor I/O, not CPU time. This is a short
measurement on this machine, not a cross-machine performance guarantee.

Strict clippy, Rust tests and JS syntax check passed. Chromium verified real
WebSocket updates, sorting, searching, the GPU filter, chart tooltips, sensor
expansion, stale detection/reconnection, missing GPU/sensors and safe rendering
of an HTML-like process name. No external web requests or JS errors. Checked
1440px desktop and 768/390/320px layouts: no page overflow, mobile process table
scrolls within its panel. Test browsers exited. The embedded source download was
reconstructed and compiled separately, offline. Screenshots and browser reports
are local artifacts in `/home/ace/baylee-data/monitor-checks/`.

No full workspace landing gates or push were performed during training.
