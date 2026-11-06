# Workstation monitor (Baylee Pulse)

`baylee-monitor` is an independent, read-only Linux workstation service. It loads
neither the game engine nor a CUDA context. The responsive German dashboard uses
embedded HTML/CSS/JavaScript: no frontend build, CDN, database or external assets.

The page has three views. **Dashboard** is a grid of cards the viewer arranges
freely. **Leistung** is a performance view after the Windows Task Manager: every
resource in a list with a live mini graph, the chosen one large with its details.
**Prozesse** is the full process table. A time range and a pause button above
them apply to every graph.

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
environments or usernames. It shows no IP or MAC address: the page has no login,
and a global IPv6 address says more about a network than a LAN page should; a
mount below `/home` is shown from `~` so it names no user, and a swap file by its
last path component. Serial numbers and UUIDs stay unparsed. The footer offers the
exact embedded source and build manifests, even before publication.

| CLI option | Default | Meaning |
| --- | --- | --- |
| `--bind` | `127.0.0.1:8787` | Explicit local/LAN socket address |
| `--allow-host` | none | Another exact `HOST:PORT` browsers may use (repeatable) |
| `--pipeline` | none | A `train_ops` pipeline directory for the training panel (repeatable, absolute, at most 8) |
| `--pipeline-root` | none | A directory searched every refresh for `train_ops` execution directories (`<root>/<experiment>/<execution>`; repeatable, absolute, at most 4); new runs appear without a restart |
| `--interval-ms` | `1000` | Sampling interval, 500–10000 ms |
| `--history-minutes` | `60` | Per-sample RAM history, 1–120 minutes; the page's ranges above one hour use the 24-hour minute ring |

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

No text is smaller than 11 px at any width; up to 640 px wide, body text is
12 px or more. On touch screens (`pointer: coarse`) of any width, buttons, sort
headers, the search field and links are at least 44 px tall, and segment
buttons and sort headers 44 px wide. The search field uses 16 px, so iOS does
not zoom on focus. A tap on a chart shows its values, as hovering does with a
mouse, and stays until the next tap elsewhere. Up to 640 px the view tabs move
to a bar at the bottom, within thumb reach, and the resource list of the Leistung
view becomes a strip that scrolls sideways. The process table scrolls inside its
panel; the page itself never scrolls sideways.

## Arranging the dashboard

"Anordnen" switches the grid into arranging mode, so no card moves by accident.
A card is moved by its head and resized by its corner, with pointer events, so a
finger works like a mouse; only a card's head takes over touch, and the page still
scrolls on its body. Escape cancels a drag. Each card's grip takes the same
actions from the keyboard: arrows move, Shift and arrows resize, Delete hides, and
a live region announces the result. Cards also have small buttons for size and
hiding. Hiding and "Standard wiederherstellen" offer an undo; hidden cards return
through the list in the settings. "Training-Ansicht" is a preset for watching a
long run.

Widths are stored on a 12-column scale. A wide monitor keeps the twelve columns,
only wider, so the layout never gains holes; a narrower grid scales them down to 8,
4 or 2 columns, and at 4 or fewer cards stack at their natural height, tiles two
to a row, and the size controls go away.

During a drag only the CSS `order` moves cards; the DOM follows at the drop.
Moving the dragged node in the DOM ends its pointer capture and the drop would
never arrive.

## Settings

Every setting lives in the browser's `localStorage`, under
`baylee-pulse.settings.v1`, so each device has its own: theme (system, light, dark),
accent colour, compact mode, data-rate unit (MiB/s, MB/s, Mbit/s), what 100 % means
for process CPU, time range, process sorting and columns, alert thresholds and the
layout. Every read and write is guarded: in a private window, or with storage
blocked, the page works and says it will forget. Nothing reaches the service.
Export and import move all of it as JSON, as a file or by pasting (for a phone).
An import is validated field by field; unknown fields are dropped and bad values
fall back to their default.

## Alerts

On 4 November 2026 the system disk ran full unnoticed and broke several jobs. A
full disk must be impossible to miss, so a critical alert shows in a red band at
the top of every view, in the tab's title and icon, in the header, and on the
disk's own row. It can be hidden for an hour; the header keeps it. "Warnungen"
lists what is active and what happened since the page opened.

| Alert | Warning | Critical |
| --- | --- | --- |
| Filesystem (not read-only), including `/tmp`, `/dev/shm`, `/run` | 85 % used; or at 50 % and full within 24 h | 95 % used; or past the warning with under 20 GiB free; or at 70 % and full within 6 h |
| Inodes | 90 % | 97 % |
| RAM | 90 % | 97 % |
| Swap | 50 % | 85 % |
| Swap traffic | over 50 MiB/s for 30 s | |
| CPU, GPU, SSD temperature | 85, 84, 70 °C | 95, 92, 80 °C |
| Pressure | memory full ≥ 10 %, I/O full ≥ 30 % | |
| GPU clocks held down by temperature or a brake | always | |
| GPU under 5 % for 5 minutes while a `baylee-*` unit runs | always | |
| Pipeline failed, or no controller | within 24 h | failed within 6 h |
| A `baylee-*` unit failed, or the OOM killer struck since the page opened | | always |
| Another system service failed; storage data over 30 s old | always | |

All percentages and temperatures are settings. An alarm that cries wolf gets
ignored, so two rules hold back. The floor in GiB only counts once a filesystem is
past the warning level: a 1 GiB `/boot/efi` is always below 20 GiB and always fine.
A forecast only counts on a fairly full disk: a large copy onto an empty disk
would fill it "in four hours" only if it never stopped. Read-only filesystems never
alarm. Short conditions wait a few seconds before they show, and an alert ends only
after 15 seconds without its condition, so it cannot flap.

Alerts are computed in the page, because thresholds are the viewer's own, and so
they need an open page: Pulse sends nothing by itself. A browser notification and
a sound are optional. Browsers allow notifications only over HTTPS or on `localhost`,
so at the LAN address the page says so, and the band, title and sound remain.

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

The training card answers "how is BayleeAI training going" without a terminal.
It is read-only like the rest of Pulse: no button in it starts, stops or changes
anything. A separate `training` thread refreshes it every five seconds, so a slow
bus or disk never delays the 1 Hz sampler, and the latest view (about 3 KB) rides
along in every sample message.

The same thread asks systemd for all loaded `*.service` units and keeps how many
are active or failed, with up to 16 failed ones by name, for the "Systemdienste"
card. Only services: a `.mount` unit's name is a path, and a path can hold a user
name.

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
`stderr.log`). Pulse shows the `--pipeline` directories and every execution directory it
finds below a `--pipeline-root`: on each refresh (the training thread, never
the 1 Hz sampler) it looks at the root, its children and its grandchildren,
at most 512 entries per directory, follows no symlink, and counts a directory
with a `status.json` file and a `jobs` directory as an execution. A new run
therefore appears, and a finished one changes, within one refresh and without
a restart. Of all candidates only `status.json` is read first (a cheap peek);
running pipelines (phase not `complete`, `failed` or `interrupted`, and no
known-dead controller) come first, then the most recently written, and the
first ten get the full read below. Per refresh and pipeline it reads:

- `status.json` (at most 64 KiB): `phase`, `updated_unix`, `promoted`, `error`
  and the recorded controller. The controller counts as running when this
  boot's `/proc/sys/kernel/random/boot_id` matches the recorded one and
  `/proc/<pid>/stat` has the recorded start time, so a recycled pid is not
  mistaken for it. A phase that is neither `complete` nor `failed` with a
  stopped controller is shown as a warning. The pipeline is linked to the
  unit its live controller runs in, read from `/proc/<pid>/cgroup` (the last
  `*.service` component), so system units such as `baylee-v6a-p1.service` and
  user units such as `baylee-v6a-ladder-gateway.service` are found without a
  bus. It also names `manifest`, the plan file.
- the plan (`manifest`, only an absolute `.json` path without `..`, at most
  256 KiB; a plain file, never a symlink): the stage names in order, which
  gives "Stufe 5 von 14" and lists the stages that have not started as
  `pending`, and from a training stage's command only `--epochs`,
  `--batch-size` and `--time-budget-seconds`. The rest of a command line is
  parsed past and dropped.
- `eval-<stage>.json` beside the plan, for each finished stage, looked up by
  name (the directory is not listed): `net_wins`, `net_losses`, `net_draws`,
  `net_win_rate` and `deals`. These files can approach a megabyte (cap
  4 MiB), so a result is parsed once and remembered by size and modification
  time until no pipeline on show asks for it any more.
- every stage directory under `jobs/` (at most 256 entries read, the newest 48
  kept): `status`, `started_at_unix`, `finished_at_unix`, `returncode` and
  `reason` from `job.json` (at most 64 KiB). Its command line and environment
  are parsed past and never kept.
- for the running stage (else the stage named by `phase`, else the newest):
  the last 8 KiB of `stdout.log` and of `stderr.log`, whichever was written
  last, and in it the newest line of the form `[tool] …`, such as
  `[train4] step=2150 epoch=1 loss=0.9757 batches_per_s=1.48`. Its
  `key=value` pairs (at most eight) become chips; the line is shown below them
  with the log's age. For a training stage the first 2 KiB of `stdout.log`
  (`train_segments=N … B=M`) and the plan's `--epochs` give the stage's total
  steps (`ceil(N / B) × epochs`); with the log's `step` and `batches_per_s`
  that is a rough "noch ca." time, cut short by `--time-budget-seconds` when
  the plan has one, and left out when the log is older than ten minutes or
  the stage is no longer running. Eval stages print their result only at the
  end, so they get an elapsed time and, once finished, their `net_wins` but no
  estimate.
- for up to four failed or quarantined stages (newest first): the last
  non-blank line of `stderr.log`, else of `stdout.log` (last 4 KiB), shown on
  the stage with paths reduced.

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
system provider returns a startup error. sysinfo now supplies only CPU usage, the
process list and uptime/load; everything else is read from `/proc` and `/sys` by
small parsers in `src/procfs/`, each taking the file's text.

- CPU overview is a percentage of all logical CPUs; process CPU uses 100% per
  logical CPU and can exceed 100%, unless the viewer chooses 100 % = all CPUs.
  Physical cores and threads are separate. Startup waits one sampling interval
  for a second CPU observation. User, system, I/O wait, interrupt and steal
  shares and the task counters come from `/proc/stat`, each core's clock and the
  governor from `cpufreq`. The per-core strip over time is kept by the browser,
  at most five minutes and only since the page opened.
- System RAM uses OS available-memory accounting (`MemTotal - MemAvailable` from
  `/proc/meminfo`). The composition bar shows used, reclaimable (available minus
  free) and free, which sum to the total. Process RAM is RSS, including shared
  pages, so its sum is not total RAM use. Disk I/O is bytes per elapsed monotonic
  second where `/proc` allows reading it. Restricted processes may return zero
  CPU/RSS/I/O from the unprivileged OS provider.
- Swap: total, used, free and cached from `/proc/meminfo`, each device from
  `/proc/swaps`, swap-in/out in bytes per second from `pswpin`/`pswpout` in
  `/proc/vmstat` at the system's page size. Swap per process is `VmSwap` from
  `/proc/<pid>/status`.
- Pressure ("Wartezeiten") comes from `/proc/pressure`: the share of time at
  least one task (`some`) or all tasks (`full`) lost waiting. High pressure means
  the machine is held back, not merely busy. Kernels without it give `null`.
- Per-process network traffic is not shown, because it does not exist in `/proc`
  without privileges or eBPF. The table shows open sockets instead, counted from
  `/proc/<pid>/fd`, which is closed for other users' processes: those read `—`,
  never zero. A pass looks at no more than 50,000 descriptors.
- GPU: dynamic NVIDIA NVML reads utilization, memory-controller load, VRAM, board
  watts, power limits, fans, temperature and thresholds, clocks and their maxima,
  the performance state, throttle reasons, encoder and decoder load, the PCIe link
  as negotiated and as possible, PCIe throughput and replays, and compute/graphics
  processes, without spawning `nvidia-smi`. Duplicate PIDs across the two lists
  count once per device. All GPUs appear in the Leistung view and hardware card;
  overview GPU/VRAM charts use device 0. Power sums readings across all GPUs and is
  unknown if one does not answer. AMD GPU/NPU telemetry is not implemented yet.
- `libnvidia-ml` itself maps `libcuda` at `nvmlInit`, opens `/dev/nvidia-uvm` and
  starts a driver thread named `cuda…`. That was so before this page too. Pulse
  creates no CUDA context: it never appears in NVML's process list and holds no
  VRAM.
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
- Network: per interface, rates in bytes and packets, totals since the interface
  appeared, errors and drops from `/proc/net/dev`; kind, state, link speed, duplex
  and MTU from `/sys/class/net`. Plots and totals sum physical interfaces, excluding
  loopback/bridges/veth to avoid double counting; virtual interfaces appear on
  request. The list stops at 32, physical ones first.
- Some drivers refresh their counters on a timer of their own, about every 1.04 s
  on this machine's `atlantic` NIC, so a 1 Hz reading finds nothing new every
  twenty-odd seconds and double the next time, which draws stalls that never
  happened. A busy interface whose counters did not move at all therefore keeps its
  rate for one reading, and the next one covers both intervals. Totals stay exact,
  and a real stop shows one reading later.
- Sensors: temperatures, fans, voltages, currents and power from
  `/sys/class/hwmon`, with the limits a driver publishes. Three drives all call
  themselves `nvme`, so a drive's chip is named after its controller. The −40 °C
  placeholders of unplugged probes fall under the rule above.

### Disks

Physical disks come from `/sys/block`: anything with a `device` link and a size,
so loop, RAM and device-mapper devices are not disks. Each has its model, size,
transport (from its path through sysfs, since a USB enclosure shows as `sdX`), its
first temperature sensor and, for flash drives, the PCIe link. Removable disks are
shown and marked.

Under each disk are its partitions with device name, filesystem label, GPT
partition name, filesystem type, mount point, used/free and read/write rates.
Labels and types come from the udev database in `/run/udev/data/b<major>:<minor>`,
the source behind `/dev/disk/by-label`, which also names unmounted partitions.
Mounts map to partitions by device number from `/proc/self/mountinfo`: the same
table as `/proc/mounts`, but its numbers survive `/dev/mapper` and `by-uuid` names.
A mount on a stacked device (LUKS, LVM) counts for the partition below it, one level
deep; a filesystem on a bare disk gets a row of its own; a bind mount does not
replace the filesystem's root and counts as "+1 Mounts". The filesystem type of a
mounted partition is the kernel's driver (`ntfs3`), else udev's (`ntfs`).

Filesystems on no listed partition still count, because they break jobs just the
same: `/tmp`, `/dev/shm` and `/run`, ZFS pools and unusual root devices appear as
"Weitere Dateisysteme". Network filesystems are never asked, because `statvfs` on a
dead server hangs.

Usage counts like `df`: used out of used plus available, so reserved blocks count
as neither. Each filesystem keeps half an hour of free-space samples; a least
squares fit gives the fill rate after five minutes, and a time until full when it
fills faster than 16 KiB/s. Rates and busy time per disk and partition come from
`/proc/diskstats` at the sampler's pace. Busy time is milliseconds with I/O in
progress per second, capped at 100 %; on a fast SSD with deep queues, 100 % does not
mean saturated.

## Transport and cost

One blocking sampler supplies all clients at 1 Hz. It reads only what cannot block
on hardware (`/proc` and the process list) and takes the rest from background
threads through `background::Latest`, so a slow sensor, a sick disk or a hung
driver delays only its own value:

| Thread | Every | Reads |
| --- | --- | --- |
| `telemetry` | interval | `/proc`, sysinfo, serialization |
| `gpu` | interval | NVML; a view older than five intervals counts as no answer |
| `sensors` | 2 s | hwmon, cpufreq |
| `processes` | 3 s | `/proc/<pid>/status` and `/fd`: threads, swap, sockets |
| `storage` | 5 s | `/sys/block`, mountinfo, udev, `statvfs` |
| `links` | 5 s | `/sys/class/net`; reading `speed` asks the driver |
| `training` | 5 s | systemd over D-Bus, pipelines |

Sensor and storage views carry the time they were taken, and the page raises an
alert when storage data goes stale. No per-thread process scan or parallel scan
workers. HTTP uses one async thread. Full current process lists are serialized once
per tick, straight from the snapshot without an intermediate JSON tree. A history
point holds rounded aggregates plus compact arrays per disk, interface and GPU.
Beside the samples of `--history-minutes`, `History` keeps one mean point per minute
for 24 hours; a missing reading stays missing in the mean, so a gap never averages
into zero. While a page stays open it adds its newest sample once per minute to the
minute ring it received, so a long-open page shows spot values for recent minutes;
a reload brings the means. History is in RAM and resets on restart. No telemetry
files are written.

`GET /api/snapshot` returns `{"type":"sample","snapshot":…,"training":…}`, or 503
at startup. `snapshot` holds `point`, `uptime_secs`, `load`, `cores`, `cpu`, `memory`,
`swap`, `pressure`, `gpus`, `gpu_status`, `gpu_driver`, `gpu_cuda`,
`cpu_power_status`, `sensors`, `storage` (`disks` with their `partitions`, `volumes`,
`problem`, `checked_ms`), `networks`, `sockets`, `processes` and `sample_ms`.
`training` holds `checked_ms`, `units` (name, `active_state`, `sub_state`, `result`,
`main_pid`, `active_since_ms`, `inactive_since_ms`, `gone_ms`), `units_problem`,
`services` (`active`, `failed`, `total`, `failed_names`) and `pipelines` (name,
`problem`, `phase`, `updated_ms`, `promoted`, `error`, `controller_pid`,
`controller_alive`, `stages`, `progress`). `GET /ws` sends
`{"type":"hello","facts":…,"history":…,"minutes":…}`, then sample messages. In a
point, `disks` entries are `[name, read, write, busy]`, `nets` `[name, rx, tx]`,
`gpus` `[index, load, vram %, watts, temperature]`.
The UI deduplicates timestamps overlapping between hello and the first sample.
Timestamps are Unix milliseconds; rate calculations use monotonic time.

At most 12 concurrent WebSocket viewers; extra upgrades receive 429. Latest-value
channels replace old snapshots, never queue them. Slow writes time out after five
seconds; the hello, which carries the history, gets 30 for a phone on weak Wi-Fi.
Incoming application messages close the socket; there are no commands. Browser
reconnect uses backoff, stale measurements are marked, background tabs stop drawing,
and gaps in chart history remain gaps. Long ranges keep each pixel column's minimum
and maximum, so a one-second spike survives an hour on 400 pixels. Pointer inspection
shows every series with its time; arrow keys do the same on a focused chart.

The page is ES modules in `web/js/`, listed with every source file in `src/assets.rs`,
which serves them and fills the source download. A test fails if a file in `src/` or
`web/` is missing from it. JavaScript reads a renamed field as `undefined` and shows a
dash without any error, so another test checks every `.snake_case` name in the page
against the JSON.

Checks: `cargo test -p baylee-monitor`,
`cargo clippy -p baylee-monitor --all-targets -- -D warnings`, and for each module
`node --input-type=module --check < crates/baylee-monitor/web/js/main.js`. Tests cover
history/live updates, the minute ring and the size of a full history, connection
limits, command rejection, permit release, cross-origin/rebinding rejection,
`--allow-host` parsing and admission, the plain-text refusal over HTTP and WebSocket,
waiting for an unassigned bind address, D-Bus marshalling (both byte orders, oversized
and broken messages), remembered finished units, service counts, pipeline reading
(stages, progress line, bounded log tail, no symlinks followed, no command line or
environment in the output, path reduction), energy-counter wrap, every `/proc` parser
on fixture text, fake sysfs trees for disks, sensors and links, mounts-to-partition
mapping, the fill forecast, rates and counter resets, that a stuck background source
holds no lock, and the JSON the page reads, including that no command line, user or
address key is on the wire. Two ignored tests talk to the real system bus:
`cargo test -p baylee-monitor -- --ignored`. Browser checks cover desktop/mobile
layout, arranging by mouse, touch and keyboard, filter/sort, reconnection and missing
sensors; close test browsers afterward.

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

## Task manager, disks and alerts (4 November 2026)

Measured on the `tools` build with three pipelines, on a machine in use (GPU
training, a large transfer, builds), with history a few minutes old:

| | Before | Now |
| --- | --- | --- |
| Sampling, wall time | about 100 ms | 16 ms |
| CPU, no viewer | | 2.75 % of one CPU over 60 s |
| CPU, one viewer | 2.4 % | 2.9 % of one CPU over 60 s |
| Resident memory | about 20–30 MB | 28 MB, peak 31 MB with 12 viewers joining at once |
| Threads | 4 | 9, of `TasksMax=16` |
| Sample message | about 40 KB | 105 KB at 425 processes |

A history point is about 450 bytes, so a full hello (3600 samples, 1440 minutes) is
about 2.3 MB; a test keeps it under 4 MiB. Worst case, twelve viewers each hold one
copy while it is sent. `MemoryMax=192M` stays.

Headless Chromium 156 (Playwright, with the libraries above) ran 106 checks against
this build: arranging by mouse, touch and keyboard, cancelling, resizing, hiding,
undo, persistence over reload, export and import with bad input, blocked
`localStorage`, theme, density, units, range, pause, tooltips by pointer and keys,
process sorting, filtering, grouping, columns and details, the Leistung view, the
alert band with its title, icon and row, reconnection, missing GPU, sensors, swap and
disks, HTML in names, WCAG AA text contrast for all accents in both themes, and at
1440, 768 (touch), 390 and 320 px no sideways scroll, no text under 11 px (12 px on
phones) and no touch target under 44 px. No JS errors, no external requests.
Screenshots were read at 2560, 1440 (dark and light), 820 and 390 px. Test browsers
and servers were closed and checked gone; the installed service was not touched.

The first screenshots caught what no check had: the 20 GiB floor raising the red
band on a nearly empty `/boot/efi`, hence the rule above, and the tab bar on phones
stuck behind the header, because a `backdrop-filter` makes an element the containing
block of its fixed children.
