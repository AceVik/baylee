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
environments or usernames. Requests require the configured numeric Host and a
matching Origin when present. Use the numeric IP, not a hostname alias. The footer
offers the exact embedded source and build manifests, even before publication.

| CLI option | Default | Meaning |
| --- | --- | --- |
| `--bind` | `127.0.0.1:8787` | Explicit local/LAN socket address |
| `--interval-ms` | `1000` | Sampling interval, 500–10000 ms |
| `--history-minutes` | `60` | RAM history, 1–120 minutes; UI shows up to one hour |

No service-specific environment variables. The optional user unit is
`scripts/systemd/baylee-monitor.service`. Install the binary in
`~/.local/lib/baylee-monitor/` and the unit in `~/.config/systemd/user/`, adjust its
explicit LAN address if desired, then run `systemctl --user daemon-reload` and
`systemctl --user enable --now baylee-monitor`. Nice 10, low CPU/IO scheduling
weight, 192 MiB memory limit, restart on failure. It requires a running user
manager; installation does not enable lingering or change root permissions.
After updates, reinstall the binary and restart the unit. Stop with
`systemctl --user stop baylee-monitor`.

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
rejection and energy-counter wrap. Browser checks cover desktop/mobile layout,
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
