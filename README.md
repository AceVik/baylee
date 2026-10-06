<p align="center">
  <b>English</b> · <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="docs/images/readme/hero.webp" alt="Baylee's front door: a moonlit conservatory garden with the gateway list" width="100%">
</p>

<h1 align="center">Baylee</h1>

<p align="center">
  A rules-enforcing engine and 3D table for playing Magic: The Gathering with friends and against a house AI.<br>
  Free, open source (AGPL-3.0), unofficial fan content.
</p>

<p align="center">
  <a href="https://github.com/AceVik/baylee/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/AceVik/baylee/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/AceVik/baylee/releases"><img alt="latest pre-release" src="https://img.shields.io/github/v/release/AceVik/baylee?include_prereleases&label=pre-release"></a>
  <a href="LICENSE"><img alt="license: AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-blue"></a>
</p>

> [!NOTE]
> **Baylee is in closed beta.** The current build is
> [v0.1.0-beta.2](https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.2), a pre-release.
> The public gateway, **[CLOSED BETA] Baylee Sanctuary** at `https://baylee.acevik.de`,
> takes new players only with a closed-beta key. Without a key you can
> still play offline against the house AI.

## Contents

- [What Baylee is](#what-baylee-is)
- [Screenshots](#screenshots)
- [Installing](#installing)
  - [Windows](#windows)
  - [macOS](#macos)
  - [Linux](#linux)
  - [Phones, tablets and the browser](#phones-tablets-and-the-browser)
  - [Checking the download](#checking-the-download)
  - [Updating](#updating)
- [Getting in: closed-beta keys](#getting-in-closed-beta-keys)
- [Playing](#playing)
- [Reporting a problem](#reporting-a-problem)
- [Running your own gateway](#running-your-own-gateway)
- [Building from source](#building-from-source)
- [Playing against a language model](#playing-against-a-language-model)
- [How it fits together](#how-it-fits-together)
- [Contributing](#contributing)
- [License and legal](#license-and-legal)

## What Baylee is

Baylee is a Magic: The Gathering platform written in Rust. It has two halves.

**A rules engine that enforces the game.** The engine applies the
Comprehensive Rules: priority and the stack, the layer system, replacement
effects, triggers, state-based actions and combat. The client never decides
what is legal. At every step the engine lists the choices a player has, and
the client offers exactly those. The engine is **deterministic**: a seeded
random-number generator, no clock and no hash-map order in the rules, so the
same game with the same answers always ends the same way. Every hosted game
is recorded move by move, and the record can be replayed to the same state.

**A 3D table** built with [Bevy](https://bevyengine.org/). It draws the
battlefield, hands, stack and graveyards, and a lobby for accounts, decks and
rooms.

Some things that set it apart:

- **Hidden information is hidden by construction.** A player's client is sent
  only what that player may see: libraries and other players' hands arrive as
  counts, and a face-down card carries no identity unless the player is
  entitled to it. A modified client cannot peek, because the data never
  reaches it.
- **Two to eight seats.** Duels, free-for-all tables and team tables. The
  host puts chairs on sides, and the sides need not be even (3 v 2 works).
  Teams keep separate turns and life totals; this is not Two-Headed Giant.
  Teammates can choose to show each other their hands.
- **Commander and freeform.** A deck that names a commander plays Commander,
  and any other deck plays freeform. Banlists and format-legality checks do
  not exist yet.
- **A house AI at five levels**: novice, casual, steady, sharp and expert
  ([docs/house-ai.md](docs/house-ai.md)). It plays from the same view a
  human gets.
- **One process per game.** A game runs in its own engine process, which a
  separate agent starts. The gateway that handles accounts and rooms runs no
  rules itself.
- **Card text in 19 languages** when the gateway has the card catalog. The
  interface is available in English and German.

The card pool grows batch by batch; this build knows about 2,700 cards. The
deck builder can show only playable cards, and it warns when a deck holds
cards that are not fully implemented yet and will not play as printed.

## Screenshots

The screenshots show Baylee's **text view**, which draws each card from its
rules text instead of the printed card image ([License and legal](#license-and-legal)
explains why). In the game, the card images come from Scryfall, and `T`
switches between the two views. The screenshots were taken with version
0.1.0-beta.2 against a local test gateway with made-up players; the picture at the top and
the sign-in panel show the real Baylee Sanctuary gateway.

<table>
  <tr>
    <td width="50%"><img src="docs/images/readme/front-door.webp" alt="Sign-in panel of the closed-beta gateway"><br><sub><b>Signing in.</b> The closed-beta gateway asks new players for a key; returning players sign in with username and password. The fan-content notice and the source link stay at the bottom.</sub></td>
    <td width="50%"><img src="docs/images/readme/lobby.webp" alt="Lobby with a deck and open tables"><br><sub><b>The lobby.</b> Your decks on the left, open tables on the right, and <i>Play the house</i> for a quick game against the AI.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/room.webp" alt="A room being arranged: seats, AI levels, starting life"><br><sub><b>Arranging a table.</b> The host sets chairs, starting life and mulligans, and gives each AI chair a level from novice to expert.</sub></td>
    <td width="50%"><img src="docs/images/readme/deck-builder.webp" alt="Deck builder with statistics and card search"><br><sub><b>The deck builder.</b> Commander, main deck and sideboard, mana curve and land statistics, and a search over the whole pool. (The picture slots are empty here because card scans were switched off for these screenshots.)</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/table-duel.webp" alt="A duel in progress, card preview in text view"><br><sub><b>A duel</b> against the house AI. Hovering a card shows it large; here in the text view.</sub></td>
    <td width="50%"><img src="docs/images/readme/game-log.webp" alt="The game log beside the table"><br><sub><b>The game log</b> (<kbd>L</kbd>) reads like a chat, with each card name a link to its preview.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/table-ring.webp" alt="A four-seat table"><br><sub><b>Four seats.</b> Larger tables sit in a ring around the felt.</sub></td>
    <td width="50%"><img src="docs/images/readme/report.webp" alt="Report form with opt-in checkboxes"><br><sub><b>Reporting a problem</b> (<kbd>F8</kbd>). Every extra piece of data is its own checkbox, and all of them start unticked.</sub></td>
  </tr>
</table>

## Installing

Download Baylee for your system from the
**[releases page](https://github.com/AceVik/baylee/releases)**. Pre-releases
are listed there too. Each system has an **installer**, which is the easy
way, and an **archive** with the same game to unpack anywhere. Baylee
updates itself wherever it may write its own folder
([Updating](#updating)).

| System | Installer | Archive |
| --- | --- | --- |
| Windows, Intel/AMD 64-bit | `Baylee-Setup-<version>-x64.exe` | `baylee-client-<version>-x86_64-pc-windows-msvc.zip` |
| Windows on ARM | `Baylee-Setup-<version>-arm64.exe` | `baylee-client-<version>-aarch64-pc-windows-msvc.zip` |
| macOS, Apple silicon (M1 and later) | `Baylee-<version>-aarch64.dmg` | `baylee-client-<version>-aarch64-apple-darwin.zip` |
| Linux, x86-64 | `Baylee-<version>-x86_64.AppImage` or `baylee_<version>_amd64.deb` | `baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, ARM64 | `Baylee-<version>-aarch64.AppImage` or `baylee_<version>_arm64.deb` | `baylee-client-<version>-aarch64-unknown-linux-gnu.tar.gz` |

Every archive contains the game, its `assets` folder, `LICENSE`, `NOTICE` and
a `README.txt` with these first-start steps.

The builds are **not code-signed**, because signing costs money. Windows and
macOS therefore warn you the first time you start Baylee. The steps below get
you past the warning.

### Windows

1. Run `Baylee-Setup-<version>-x64.exe` (or `-arm64` on Windows on ARM).
2. If Microsoft Defender SmartScreen says "Windows protected your PC", click
   **More info** and then **Run anyway**.
3. The setup installs Baylee for your user only, in
   `%LOCALAPPDATA%\Programs\Baylee`, and asks for no administrator rights.
   It adds Baylee to the Start menu and, if you tick the box, to the
   desktop.

To remove it, use **Settings → Apps**. Downloaded updates and cached card
images stay in `%LOCALAPPDATA%\baylee`; delete that folder too to remove
everything.

Without the setup: unpack the zip to any folder, for example
`Documents\Baylee`, keep the `assets` folder next to `baylee-client.exe`,
and start `baylee-client.exe`.

The x86-64 build needs a CPU with x86-64-v2 (SSE4.2, POPCNT); Windows 11
24H2 does not start without one either.

### macOS

Only **Apple silicon** Macs (M1 and later) are supported for now; there is no
Intel build. macOS 11 or later is required.

1. Open `Baylee-<version>-aarch64.dmg` and drag **Baylee** onto
   **Applications**. Then eject the disk image.
2. Open Baylee from Applications. The first time, macOS refuses because the
   app is not notarised.
3. Open **System Settings → Privacy & Security**, scroll down to the message
   about Baylee and click **Open Anyway**.

Start it from Applications, not from the disk image: from there it cannot
update itself. With the zip instead, unpack it and move `Baylee.app` to
Applications (or anywhere else) the same way.

Or, in Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Baylee.app
```

### Linux

The **AppImage** is one file that runs on its own:

```bash
chmod +x Baylee-<version>-x86_64.AppImage
./Baylee-<version>-x86_64.AppImage
```

The **.deb** (Debian, Ubuntu and their relatives) installs Baylee in
`/opt/baylee` with a menu entry, and the command `baylee`:

```bash
sudo apt install ./baylee_<version>_amd64.deb
```

Or unpack the archive:

```bash
tar -xzf baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz
cd baylee-client-<version>-x86_64-unknown-linux-gnu
./baylee-client
```

You need a Vulkan driver and the ALSA, udev, X11/Wayland and xkbcommon
runtime libraries; the `.deb` names them, so `apt` installs them for you,
and the AppImage uses the system's like the archive does. On Debian or
Ubuntu these are
`libasound2 libudev1 libxkbcommon-x11-0 libwayland-client0 libvulkan1`. The
binaries are built on Ubuntu 22.04 and run on any distribution with glibc
2.35 or newer. Like the Windows one, the x86-64 build needs a CPU with
x86-64-v2.

### Phones, tablets and the browser

There are **no mobile builds to download yet.**

- **Android:** the client builds and runs on a real phone
  (`scripts/mobile/android-build.sh`), but the lobby is not usable by touch
  yet, so you cannot sign in on the phone.
- **iOS:** the client runs only in the simulator
  (`scripts/mobile/ios-sim-run.sh`).
- **Browser:** play in the browser at **<https://baylee.acevik.de/play/>**,
  on the Sanctuary gateway; a new account there still needs a
  [closed-beta key](#getting-in-closed-beta-keys). The page needs WebGPU:
  Chrome or Edge 113 or later, Safari 26 or later (macOS, iOS, iPadOS), or
  Firefox 141 or later on Windows and 145 or later on macOS. Firefox on
  Linux and Android does not offer WebGPU yet, and a browser without it is
  told so instead of shown a blank page. The browser build plays only on the
  gateway that serves it; for other gateways use the desktop client.

[docs/mobile.md](docs/mobile.md) describes exactly what works and what does
not.

### Checking the download

Next to each installer and archive on the release page there is a `.sha256`
file, and the release notes list the installers' checksums too. Download the
file and its `.sha256` into the same folder and run:

```bash
# Linux
sha256sum -c Baylee-<version>-x86_64.AppImage.sha256
# macOS
shasum -a 256 -c Baylee-<version>-aarch64.dmg.sha256
```

On Windows, run
`certutil -hashfile Baylee-Setup-<version>-x64.exe SHA256` in PowerShell
and compare the result with the number in the `.sha256` file.

### Updating

Baylee updates itself. It asks GitHub for a newer release at start and every
six hours, downloads it, checks its signature, and installs it when you quit;
the lobby's corner and the table's menu say "Update X ready – installs when you
quit", and the next start says "Updated to X" once. An update whose signature
does not verify is never installed. The settings screen has two switches,
"Update automatically" and "Check for updates automatically" (off: Baylee asks
GitHub nothing until you press "Check for updates"); `docs/privacy.md` says
what GitHub sees.

Baylee only links to the release page, and you update by hand, when it
cannot replace itself: its folder is not writable for your user (it never
asks for administrator rights; the `.deb`'s `/opt/baylee` is such a folder,
and so is a running AppImage), or macOS runs it from a read-only copy
because `Baylee.app` was started from the disk image or from where it was
unpacked in Downloads (move it once, to Applications for example). To update
by hand, run the new installer over the old installation (the `.deb` with `sudo
apt install`, the AppImage by replacing the file), or download the new
archive and replace the old folder (or `Baylee.app`) with the new
one.

Replacing the program keeps your settings, because they are not stored next
to it. On Windows, the client keeps them in `%APPDATA%\Baylee`; on Linux
and macOS, in `~/.config/baylee/`. Setting `$XDG_CONFIG_HOME` overrides
that location with `$XDG_CONFIG_HOME/baylee` on all three systems. The files are:

- `client-settings.json`: language, gateway list, report consent, and a
  guest's session;
- `preferences.json`: key bindings and standing answers;
- `offline-decks.json`: decks built offline.
- `update.json`: the two update switches.

Card images are cached separately, in `~/Library/Caches/baylee` (macOS),
`%LOCALAPPDATA%\baylee` (Windows) or `~/.cache/baylee` (Linux). Your account
and the decks you built online are stored on the gateway, not on your
computer. When you sign in, your key bindings and preferences are loaded from
the gateway.

Every build shows its version and commit in the lobby. A new minor version
(0.1 → 0.2) may be unable to talk to an older gateway, so update when a new
release comes out.

## Getting in: closed-beta keys

The Baylee Sanctuary gateway is a closed beta. **A new account and a new
guest each need a key** of the form `BAYLEE-XXXX-XXXX-XXXX-XXXX`. Keys are
handed out by the project owner ([@AceVik](https://github.com/AceVik)); there
is no public sign-up form.

With a key:

1. Start Baylee. The front door already lists **Baylee Sanctuary**; choose it.
2. Choose **Create account**. Pick a *username* (3–24 letters, digits, `_ - .`), a
   *display name* and a password, and paste the key. Upper and lower case,
   spaces and dashes in the key do not matter.
3. Or choose **Play as guest**: a display name and a key are enough. A guest can
   build decks and play, but cannot upload sleeves or playmats. A guest is
   deleted when it signs out or after 30 days without use.

Signing up **asks for no e-mail address**: an account is a username and a
password. The username is private: other
players see only your display name and a short tag, such as `Alice#af03`.
Once you have an account, you sign in with your username and password and need
no key. You can delete the account from the client at any time
(`DELETE /account`).

## Playing

**Offline**, without any gateway: choose *Play offline* on the front door.
You get the same lobby with your offline decks, the deck builder and a
table of house-AI opponents. *Play the house* starts a duel right away with
the two bundled decks.

**Online**, signed in to a gateway:

- **Decks.** Build a deck in the deck builder: search the pool (optionally
  only cards that are fully playable), set main deck, sideboard and commander,
  watch the mana curve, and pick a printing for each card. A deck is stored as
  one text line per card, such as `1 Lightning Bolt (M11) 149`
  ([docs/deck-format.md](docs/deck-format.md)).
  The gateway keeps each deck's version history. A registered account can give
  a deck its own sleeve and playmat.
- **Rooms.** Open a room with 2 to 8 chairs and, if you like, a name and a
  password. As host you decide who sits where: another player, or the house AI
  at one of its five levels. You also set the sides, the starting life, free
  mulligans, starting permanents and the clock (`casual`, `standard`, `blitz`
  or `untimed`). Every player chooses their own deck and says *ready*, and the
  host starts the game. When it ends, the table can play again.
- **At the table.** The engine asks and you answer. You can play with the
  mouse or entirely with the keyboard:

| Key | Does |
| --- | --- |
| <kbd>Space</kbd> | Confirm / pass priority |
| <kbd>Enter</kbd> | Act on the card under the cursor, else pass |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>, <kbd>E</kbd> | Move the card cursor; play or select the card under it |
| <kbd>Esc</kbd> | Cancel (step by step) |
| <kbd>Tab</kbd> / <kbd>⇧ Tab</kbd> | Fast-forward to the next phase / next turn (decisions stay yours) |
| <kbd>F6</kbd> / <kbd>F7</kbd> | Let the stack resolve / nothing more this turn |
| <kbd>K</kbd> / <kbd>B</kbd>, <kbd>Y</kbd> / <kbd>N</kbd> | Mulligan: keep / put a card on the bottom; yes / no |
| <kbd>T</kbd>, hold <kbd>Cmd</kbd>/<kbd>Alt</kbd> | Card text instead of art (switch on/off / only while held) |
| <kbd>G</kbd>, <kbd>L</kbd> | Zone browser (graveyards, exile, stack), game log |
| <kbd>F8</kbd> | Report a problem |

Every key can be rebound in the settings, and a signed-in account keeps its
bindings on the gateway. [docs/keyboard-map.md](docs/keyboard-map.md) has the
complete list, including combat and the ability sheet.

## Reporting a problem

Press <kbd>F8</kbd> at the table or in the lobby, or use the report button in
the menu. A report is sent to the gateway you are signed in to. It always
includes its kind, your text, the client version and, at a table, the game's
id. The gateway adds its own record of that game, in which seats are numbers
and no player is named.

Anything more is **opt-in, one checkbox each**, and every checkbox starts
unticked: system and hardware, the table as you see it, your game log (with
players renamed to "Player A", "Player B", …), your settings, and a
screenshot. Session tokens are never sent. After a crash, the next start asks
once whether crash reports may be sent. If you are not signed in to any
gateway, nothing is sent.

[docs/privacy.md](docs/privacy.md) lists everything the client, the gateway
and the feedback service keep.

## Running your own gateway

A gateway is the server players sign in to. A game runs as three kinds of
process:

- **`baylee-gateway`** handles accounts, decks, rooms and routing. It runs no
  rules and needs PostgreSQL.
- **`baylee-agent`** starts one engine process per game when the gateway
  asks for one. It knows no rules either.
- **`baylee-engine-server`** is one game. It dials back to the gateway.

This section is a guide. The normative references are
[docs/protocol.md](docs/protocol.md) and the *Environment* section of
[CLAUDE.md](CLAUDE.md#environment).

### 1. Database

```bash
docker compose up -d          # PostgreSQL 18 from compose.yaml
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
```

Change the password for anything that is reachable from outside. The gateway
creates and migrates its tables itself.

### 2. Build and start

```bash
cargo build --release -p baylee-gateway -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)   # shared by gateway and agent
RUST_LOG=info BAYLEE_GATEWAY_NAME="My table" ./target/release/baylee-gateway   # listens on 0.0.0.0:28766
RUST_LOG=info ./target/release/baylee-agent        # finds baylee-engine-server beside itself
```

Without `RUST_LOG=info` the servers log nothing. Without a connected agent,
the gateway can list rooms but cannot start a game (`503`).

When gateway and agent run on one machine, they can talk over a **unix
socket** instead of TCP. Set `BAYLEE_UNIX_SOCKET=/run/baylee/gateway.sock`
for the gateway and `BAYLEE_GATEWAY=unix:/run/baylee/gateway.sock` for the
agent. See
[docs/protocol.md § On the same machine](docs/protocol.md#on-the-same-machine-the-unix-socket).

Never expose `baylee-engine-server` on its own: without the gateway's
arguments it is an unauthenticated development harness.

### 3. Settings that matter

| Variable | What it does |
| --- | --- |
| `DATABASE_URL` | Required. |
| `PORT` | Port to listen on (default `28766`). |
| `BAYLEE_AGENT_TOKEN` | The agent's secret. Without it, no agent can connect. |
| `BAYLEE_GATEWAY_NAME` | The name clients show for the gateway. |
| `BAYLEE_REGISTRATION` | Unset: open sign-up. `invite`: closed beta, keys needed. `off`: no sign-up. |
| `BAYLEE_GUESTS=off` | Refuse guest accounts. `BAYLEE_GUEST_CAP` limits live guests (default 1000). |
| `BAYLEE_TRUSTED_PROXIES` | Address of your reverse proxy, so rate limits see the real client IP. |
| `BAYLEE_ART_PATH` / `BAYLEE_DECK_IMAGE_PATH` | Where card art is cached and uploaded sleeves/mats are stored (`off` disables them). |
| `BAYLEE_ENGINE_URL` | Set it when the agent runs on another machine. |
| `BAYLEE_SOURCE_URL` | Where your build's source is published, if you changed it (AGPL §13). |
| `BAYLEE_FEEDBACK_URL`, `…_TOKEN`, `…_KEY` | Optional: forward <kbd>F8</kbd> reports to a feedback service. |

The gateway answers `GET /source` with its licence, version and commit
without asking for sign-in. The AGPL requires you to offer your source to the
people who use your server, and this route does that for you.

### 4. Closed-beta keys

With `BAYLEE_REGISTRATION=invite`, you create keys on the server:

```bash
baylee-gateway invite create --uses 1 --expires 30d --note "Max"
baylee-gateway invite list
baylee-gateway invite revoke <id>
```

The command only needs `DATABASE_URL`; the gateway does not have to be
running. Each key is printed once and only its hash is stored. On a server
set up with `scripts/server/baylee-deploy`, the wrapper
`sudo baylee-invite …` reads the gateway's settings for you.

### 5. Card text (optional)

Without a catalog, games still run: cards show their English Oracle text,
which is built into the programs. For searchable card text in 19 languages,
load Scryfall's bulk data into the same database:

```bash
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest                 # all languages, ~390 MB download
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest --english-only  # ~80 MB
```

### 6. HTTPS in front

The gateway speaks plain HTTP and WebSocket, so put a TLS proxy in front of
it. The repository ships no proxy configuration; this minimal
[Caddy](https://caddyserver.com/) example is a starting point:

```caddyfile
baylee.example.org {
    reverse_proxy 127.0.0.1:28766
}
```

Caddy obtains a certificate, passes WebSockets through and sets
`X-Forwarded-For`. Then set `BAYLEE_TRUSTED_PROXIES=127.0.0.1` on the
gateway. The gateway authenticates with bearer tokens and sets no cookies,
so no further CORS or cookie configuration is needed.

### 7. Bug reports (optional)

`baylee-feedback` is a separate service that collects the reports gateways
forward, with a small web UI for reading them. Setting it up is described in
[docs/feedback.md § Running it](docs/feedback.md#running-it). Without it, the
gateway refuses reports with "reports are not configured".

## Building from source

You need **Rust stable, 1.95 or newer** (`rust-toolchain.toml` pins the stable
channel with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target). On
Linux you also need `libasound2-dev libudev-dev libwayland-dev`.

```bash
git clone https://github.com/AceVik/baylee.git
cd baylee
cargo run --release -p baylee-client          # the game; offline play needs nothing else
```

- **Browser build:** `trunk serve index.html --release`, run from
  `crates/baylee-client/`. Always use `--release`: a debug wasm is about
  350 MB.
- **Android / iOS simulator:** `scripts/mobile/android-build.sh`,
  `scripts/mobile/ios-sim-run.sh` ([docs/mobile.md](docs/mobile.md)).
- **A local table with seats:** start a gateway and an agent (above), then
  run `cargo run -p xtask -- dev-table --seats 4 --ai sharp --play`.

For contributors, the checks CI runs:

```bash
cargo fmt --all
./scripts/gate-rules.sh                        # quick: rules crates, without the client
DATABASE_URL=… ./scripts/gate.sh               # the full gate: fmt, clippy, nextest, validate
DATABASE_URL=… ./scripts/gate-features.sh      # the non-default feature builds
```

## Playing against a language model

A chair can be played by a language model through the seat bridge
(`baylee-seat`). It sits at the table as an ordinary socket player and hands
its questions to a model: a local one in LM Studio, or one behind an
OpenAI-compatible or Anthropic API. Everything below is for a local test
table; the full reference is [docs/llm-seat.md](docs/llm-seat.md).

**1. A local gateway and agent.** Start them as in
[Running your own gateway](#running-your-own-gateway), at the default address
`http://127.0.0.1:28766`, then build the bridge:

```bash
cargo build -p baylee-seat -p xtask
```

**2. A settings file.** The bridge reads `llm-seat.json` from the client's
config directory (`~/.config/baylee/` on macOS and Linux,
`%APPDATA%\Baylee\` on Windows), or the file `BAYLEE_SEAT_CONFIG` names. Keys
never go in this file: a profile names the environment variable its key is
read from. Unknown fields are refused. An example with a local model and
DeepSeek:

```json
{
  "default": "lmstudio",
  "caps": { "day_usd": 5, "month_usd": 30, "day_tokens": 50000000, "month_tokens": 500000000 },
  "profiles": {
    "lmstudio": {
      "provider": "openai",
      "model": "google/gemma-4-26b-a4b-qat",
      "base_url": "http://127.0.0.1:1234/v1",
      "answer": "tools",
      "effort": "low",
      "game_tokens": 40000000
    },
    "deepseek": {
      "provider": "openai",
      "model": "deepseek-flash",
      "base_url": "https://api.deepseek.com/v1",
      "key_env": "DEEPSEEK_API_KEY",
      "answer": "json",
      "game_tokens": 5000000
    }
  }
}
```

- **LM Studio:** start its server (Developer tab, or `lms server start`). The
  model id is whatever `curl http://127.0.0.1:1234/v1/models` lists. LM Studio
  needs no key, but the bridge wants a non-empty one:
  `export BAYLEE_LLM_API_KEY=lm-studio`. Use `"answer": "tools"`; LM Studio
  refuses the plain JSON mode. The first question loads the model, which can
  take a minute. A 26B model on an M1 Max thinks 30–60 s a decision at full
  effort; `"effort": "low"` shortens that.
- **DeepSeek:** `export DEEPSEEK_API_KEY=…` in the shell you start the table
  from. A model the bridge has no price for plays only under `game_tokens`
  (or a `price` in its profile).
- **Claude Code on a subscription:** a profile such as
  `"cc": {"provider": "cli", "model": "claude:opus", "command": "/Users/<you>/.local/share/claude/versions/<version>", "game_calls": 300}`
  plays through the `claude` you signed in to by hand, on that login and with
  no key (`key_env`, `base_url` and `price` are refused). Pin `command` to a
  version's own file, since `~/.local/bin/claude` is a link the tool moves on
  every update. A game counts tokens (20,000,000 by default) and calls
  (`game_calls`, 500 by default); see
  [docs/llm-seat.md](docs/llm-seat.md#a-cli-as-the-model).
- **Caps:** the `caps` hold across games. Each game reserves its budget in
  `llm-spend.json` beside the settings file before it sits down, and settles
  after.

**3. Play against it.** This seats you in chair 0 (the client opens) and the
profile's model in chair 1:

```bash
cargo run -p xtask -- dev-table --bridge profile:lmstudio --play
```

- `--bridge house` seats the house AI through the same bridge, a quick check
  that the table works before a model is involved.
- `--bridge profile:deepseek` plays the DeepSeek profile, `--bridge profile:cc`
  Claude Code.
- The bridge writes what it asked and what the model answered to
  `target/seat-transcripts/`. That is the first place to look when a model
  plays strangely.
- When the model fails, times out or answers something illegal, the house
  answers that question for it, and the transcript says so.

## How it fits together

```mermaid
flowchart LR
  cardtext[baylee-cardtext] --> core[baylee-core]
  core --> engine[baylee-engine]
  engine --> gamehost[baylee-gamehost]
  view[baylee-view] --> gamehost
  ai[baylee-ai] --> gamehost
  gamehost --> engineserver[baylee-engine-server]
  core --> clientcore[baylee-client-core]
  view --> clientcore
  protocol[baylee-protocol] --> clientcore
  clientcore --> client[baylee-client<br/>Bevy]
  protocol --> gateway[baylee-gateway]
  protocol --> agent[baylee-agent]
  db[baylee-db] --> gateway
  catalog[baylee-catalog] --> gateway
```

Each arrow drops a capability. The **engine** is synchronous and pure: no
I/O, no async, no clock. **gamehost** builds each seat's view from it, and
that view hides what the seat may not see. **client-core** is the client's
logic without a renderer, and holds most of the client's tests. The
**gateway** and the **agent** link no rules at all. Cards are Rust code,
written in a small DSL, one file per card under `crates/baylee-cards/src/cards/`.

Where to read on:

| Topic | Document |
| --- | --- |
| Overall plan | [docs/architecture.md](docs/architecture.md) |
| Engine internals (layers, events, cleanup, loops) | [docs/engine-internals.md](docs/engine-internals.md) |
| Wire protocol, gateway, rooms, keys | [docs/protocol.md](docs/protocol.md) |
| The client | [docs/client.md](docs/client.md) |
| Writing cards | [docs/card-dsl.md](docs/card-dsl.md), [docs/card-identity.md](docs/card-identity.md) |
| House AI | [docs/house-ai.md](docs/house-ai.md) |
| Privacy | [docs/privacy.md](docs/privacy.md) |
| Legal | [docs/legal.md](docs/legal.md) |
| Releasing | [docs/releasing.md](docs/releasing.md) |

## Contributing

Issues and pull requests are welcome on
[GitHub](https://github.com/AceVik/baylee). Before you start, read
[AGENTS.md](AGENTS.md) and [CLAUDE.md](CLAUDE.md): they are the project's
working rules, for people and coding agents alike. A few points:

- Run the gate before you push. CI also builds for Windows, macOS, wasm32 and
  the minimum Rust version.
- Every card you add or fix gets a test that plays it, in
  `crates/baylee-engine/src/engine/card_tests/`.
- The engine stays deterministic: no `std::time`, no randomness except the
  seeded RNG, no hash-map iteration in hot paths.
- Nothing that shows a card, symbol, font or sound is merged without checking
  it against [docs/legal.md](docs/legal.md).

## License and legal

Baylee's source code is licensed under the
**[GNU Affero General Public License v3.0 only](LICENSE)**. If you run a
modified gateway for other people, you must offer them its source; the
gateway's `GET /source` route does this. The project is free and
non-commercial, and no feature will ever be sold.

**Fan content.** Baylee is unofficial Fan Content permitted under the
[Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy).
Not approved/endorsed by Wizards. Portions of the materials used are property
of Wizards of the Coast. ©Wizards of the Coast LLC. Baylee is not affiliated
with Wizards of the Coast, and no Wizards of the Coast asset is shipped in
this repository.

**Scryfall.** Card data and images are provided by
[Scryfall](https://scryfall.com). The client loads card images from Scryfall
while you play; they are not part of this repository and are not
redistributed. For that reason the screenshots above use Baylee's own text
view, which draws a card from its name and rules text in our own layout, with
no frame art or image of a printed card
([docs/legal.md](docs/legal.md), clauses 2 and 3). Baylee is not affiliated
with Scryfall.

**Third-party material.** [NOTICE](NOTICE) lists the bundled fonts (Alegreya
Sans, Faustina, Font Awesome Free and the Mana font, all SIL OFL 1.1);
[docs/legal.md](docs/legal.md) §5 covers the CC0 instrument samples the music
is played with. NOTICE also names the external,
GPL-licensed card-script corpus that code generation may read from a
developer's own checkout. No file of that corpus is ever copied into this
repository or shipped with a build.
