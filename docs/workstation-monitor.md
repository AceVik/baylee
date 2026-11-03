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
| `--pipeline` | none | A `train_ops` pipeline directory for the training panel (repeatable, absolute, at most 8) |
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

### On a small screen

No text is smaller than 11 px at any width; up to 600 px wide, body text is
12 px or more. On touch screens (`pointer: coarse`) of any width, buttons, sort
headers, the search field and links are at least 44 px tall, and segment
buttons and sort headers 44 px wide. The search field uses 16 px, so iOS does
not zoom on focus. A tap on a chart shows its values, as hovering does with a
mouse, and chart labels are 11 px below 500 px width. Below 380 px the stat
cards drop their icons. The process table scrolls inside its panel; the page
itself never scrolls sideways.

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

## Training panel

The panel under the four stat cards answers "how is BayleeAI training going"
without a terminal. It is read-only like the rest of Pulse: no button in it
starts, stops or changes anything. A separate `training` thread refreshes it
every five seconds, so a slow bus or disk never delays the 1 Hz sampler, and
the latest view (about 3 KB) rides along in every sample message.

**System units.** Pulse asks systemd over the system D-Bus
(`/run/dbus/system_bus_socket`, as its own user) for the loaded system units
matching `baylee-*`, typically transient `systemd-run` units such as
`baylee-v6x-exit1.service`, at most 24. It reads `ActiveState`, `SubState`,
`ActiveEnterTimestamp`, `InactiveEnterTimestamp` and, for services, `Result`
and `MainPID`. Unit descriptions are dropped unread: for a `systemd-run` unit
they are its command line. The client is a small, fixed D-Bus implementation in
`training/dbus.rs`: it authenticates with SASL `EXTERNAL`, calls only
`ListUnitsByPatterns` and `Properties.Get`, subscribes to no signals, caps a
message at 1 MiB and times out socket operations after two seconds. systemd
forgets a transient unit as soon as it exits successfully, so Pulse remembers,
in memory, the last eight units that disappeared within 24 hours, marked
"beendet" with the time it noticed. A failed unit stays loaded in systemd and
shows as "fehlgeschlagen" with its `Result`. User units (such as Pulse itself)
are not shown. When the bus cannot be reached, the panel says so and keeps the
previous list.

**Pipelines.** Each `--pipeline` directory is a `train_ops` execution
directory (`status.json`, `jobs/<stage>/job.json`, `jobs/<stage>/stdout.log`,
`stderr.log`). Directories are explicit configuration: Pulse reads nothing
outside them, follows no symlink below them and does not search for new ones,
so a new pipeline needs its own `--pipeline` and a restart. Per refresh and
pipeline it reads:

- `status.json` (at most 64 KiB): `phase`, `updated_unix`, `promoted`, `error`
  and the recorded controller. The controller counts as running when this
  boot's `/proc/sys/kernel/random/boot_id` matches the recorded one and
  `/proc/<pid>/stat` has the recorded start time, so a recycled pid is not
  mistaken for it. A phase that is neither `complete` nor `failed` with a
  stopped controller is shown as a warning. The pipeline is linked to the
  system unit whose `MainPID` is its controller.
- every stage directory under `jobs/` (at most 256 entries read, the newest 48
  kept): `status`, `started_at_unix`, `finished_at_unix`, `returncode` and
  `reason` from `job.json` (at most 64 KiB). Its command line and environment
  are parsed past and never kept.
- for the running stage (else the stage named by `phase`, else the newest):
  the last 8 KiB of `stdout.log` and of `stderr.log`, whichever was written
  last, and in it the newest line of the form `[tool] …`, such as
  `[train4] step=2150 epoch=1 loss=0.9757 batches_per_s=1.48`. Its
  `key=value` pairs (at most eight) become chips; the line is shown below them
  with the log's age.

Text from these files reaches the browser only after control characters are
removed, every absolute or home-relative path is reduced to `…/last-component`
and the result is cut at 240 characters. A missing directory or an unreadable
`status.json` is reported on the pipeline's card instead of failing the panel.

On a phone the panel is a single column: pipeline cards (phase, controller,
progress chips, the newest five stages with "Alle N Stufen" for the rest),
then the units with "seit", duration and PID.

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
parallel scan workers. HTTP uses one async thread; the training panel has its own
thread that wakes every five seconds. Full current process lists are serialized
once per tick, straight from the snapshot without an intermediate JSON tree;
only compact aggregate points enter the bounded history. History is in RAM and
resets on restart. No telemetry files are written.

`GET /api/snapshot` returns `{"type":"sample","snapshot":…,"training":…}`, or 503
at startup. `training` holds `checked_ms`, `units` (name, `active_state`,
`sub_state`, `result`, `main_pid`, `active_since_ms`, `inactive_since_ms`,
`gone_ms`), `units_problem` and `pipelines` (name, `problem`, `phase`,
`updated_ms`, `promoted`, `error`, `controller_pid`, `controller_alive`,
`stages`, `progress`). `GET /ws` sends `{"type":"hello","facts":…,"history":…}`,
then sample messages.
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
and WebSocket, waiting for an unassigned bind address, D-Bus marshalling (both
byte orders, oversized and broken messages), remembered finished units,
pipeline reading (stages, progress line, bounded log tail, no symlinks followed,
no command line or environment in the output, path reduction) and energy-counter
wrap. Two ignored tests talk to the real system bus:
`cargo test -p baylee-monitor -- --ignored`. Browser checks cover desktop/mobile
layout, filter/sort, reconnection and missing sensors; close test browsers
afterward.

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

## Phone access, startup and training panel (3 November 2026)

The owner could not open Pulse on a phone in the LAN. A packet capture showed
the path itself works (a LAN device at 192.168.0.64 held a live WebSocket), and
`ufw`/nftables block nothing on 8787. What remained: the bind racing DHCP at
login (fixed in-process, above), a phone using a host name and getting an
empty 403, and a browser trying `https://`. The FritzBox also answers `bigace`
and `bigace.fritz.box` with four addresses (192.168.0.80, .29, .28 and .103,
stale leases), so a phone that opens Pulse by name may reach no Pulse at all;
the numeric URL is the one to use.

Installed unit, after a backup of the previous binary
(`~/.local/lib/baylee-monitor/baylee-monitor.bak`) and unit
(`baylee-monitor.service.bak` beside it):

```ini
ExecStart=%h/.local/lib/baylee-monitor/baylee-monitor --bind 192.168.0.103:8787 --allow-host bigace.local:8787 --allow-host bigace.fritz.box:8787 --allow-host bigace:8787 --pipeline /home/ace/baylee-data/v6/train/v6x-exit/execution-x1
```

Verified after `systemctl --user restart baylee-monitor`: `GET /` on
`http://192.168.0.103:8787` is 200, `bigace.local:8787` is 200, an unknown
Host gets the plain-text 403 and one journal line, and `/api/snapshot`
carries the training view (unit `baylee-v6x-exit1.service` active, pipeline
`v6x-exit/execution-x1` in `train-X1a` with its controller alive, four stages
and the newest `[train4] step=…` line). The service used about 20 MB and four
tasks; with one browser connected it took 2.4% of one logical CPU over ten
seconds, during GPU training.

Headless Chromium (Playwright) at 1440, 768 (touch), 390 and 320 px: no
sideways scroll, no JS errors, no external requests, no text under 11 px, no
touch target under 44 px at 768/390/320 px. Before the change, 390 px had 39
text elements at 8–10 px and 15 controls under 44 px. This machine lacks the
system libraries Playwright's Chromium needs (`libatk1.0-0t64`,
`libatk-bridge2.0-0t64`, `libatspi2.0-0t64`, `libxdamage1`, `libxres1`); for
this check they were fetched with `apt-get download` and unpacked into a
scratch directory for `LD_LIBRARY_PATH`, changing nothing on the system. Test
browsers and test servers were closed and checked gone.
