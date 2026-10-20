<p align="center">
  <b>English</b> · <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="docs/images/readme/hero.webp" alt="Baylee's front door: signing in at a gateway, in a moonlit garden" width="100%">
</p>

<h1 align="center">Baylee</h1>

<p align="center">
  A free 3D table for playing Magic: The Gathering with friends, against a house AI or against a language model,<br>
  with a rules engine that knows the rules so you don't have to argue about them.<br>
  Open source (AGPL-3.0), unofficial fan content.
</p>

<p align="center">
  <a href="https://baylee.acevik.de/"><b>Website</b></a> ·
  <a href="https://github.com/AceVik/baylee/releases"><b>Download</b></a> ·
  <a href="https://discord.gg/4pP3uAwgY"><b>Discord</b></a> ·
  <a href="https://github.com/AceVik/baylee-central"><b>Accounts (Baylee Central)</b></a>
</p>

<p align="center">
  <a href="https://github.com/AceVik/baylee/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/AceVik/baylee/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/AceVik/baylee/releases"><img alt="latest pre-release" src="https://img.shields.io/github/v/release/AceVik/baylee?include_prereleases&label=pre-release"></a>
  <a href="LICENSE"><img alt="license: AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-blue"></a>
</p>

> [!NOTE]
> **Baylee is in closed beta.** This page describes version **0.1.0-beta.7**, a
> pre-release for **Windows, macOS and Linux** computers. The public server,
> **Baylee Sanctuary** at `https://baylee.acevik.de`, takes new players only
> with a closed-beta key. Without a key you can still play offline against
> the house AI, build decks, or run a server of your own for your group.

## Contents

- [What Baylee is](#what-baylee-is)
- [New in beta.7](#new-in-beta7)
- [Screenshots](#screenshots)
- [Installing](#installing)
- [Your first game](#your-first-game)
- [Playing with friends](#playing-with-friends)
- [Friends and chat](#friends-and-chat)
- [Replays](#replays)
- [Playing against a language model](#playing-against-a-language-model)
- [Reporting a problem](#reporting-a-problem)
- [Where the cards stand](#where-the-cards-stand)
- [Running your own gateway](#running-your-own-gateway)
- [Building from source](#building-from-source)
- [How it fits together](#how-it-fits-together)
- [Contributing](#contributing)
- [License and legal](#license-and-legal)

## What Baylee is

Baylee is a program for playing Magic: The Gathering on your computer. You
start it, pick a deck and sit down at a table, either alone against the
computer or with friends over the internet.

**The game knows the rules.** Baylee has a rules engine that follows Magic's
Comprehensive Rules: priority and the stack, triggers, replacement effects,
layers, combat and the many small checks in between. At every moment it
tells you what you may do, and the table offers exactly that, so nobody has
to look anything up or argue. You can play with the mouse or entirely with
the keyboard.

**It is fair by construction.** Your computer is only sent what you may see.
Your opponents' hands and the libraries arrive as counts, never as cards, so
a modified program cannot peek: the information simply is not there.

What you can do with it:

- **Play offline** against the house AI, at five levels from *novice* to
  *expert*, with no account and no internet connection.
- **Play online** on a *gateway* (a Baylee server): 2 to 8 players at one
  table, free-for-all or in teams (uneven teams such as 3 against 2 work
  too), with or without AI players in the empty chairs. A deck that names a
  commander plays Commander; any other deck plays freeform.
- **Add friends and chat** with them, end-to-end encrypted.
- **Watch** a friend's running table, and **watch your own games again** as
  replays.
- **Seat a language model** at your table, on your own key, or one the
  server's operator offers.
- **Build decks** with a search over every card in the pool, mana curve and
  land statistics, and import or export in common text formats.
- **Read the cards in 19 languages** when the gateway has the card catalog.
  The program itself speaks English and German.

Baylee is free and stays free: no paywall, no ads, nothing to buy. It is a
non-commercial fan project and not affiliated with Wizards of the Coast.

## New in beta.7

- **Friends and chat.** Friend requests, whispers, groups, a chat for each
  table and each team, and who of your friends is online and where. Every
  message is end-to-end encrypted ([details](#friends-and-chat)).
- **The game log is a tab of the chat window.** At a table the window has
  tabs for *All*, *Log*, *Table*, *Team*, *Private* and *Group*. The log
  names phases, modes and targets, and every card in it is a link to its
  picture.
- **Replays.** Every online game you played can be watched again from any
  computer, from your seat, from the table's public view, or from any
  player's seat if that player agreed to share their hidden cards
  ([details](#replays)). Games against the house are kept on your computer.
- **Watching.** A table can let others watch; the lobby shows a *Watch*
  button and how many are watching.
- **A new payment window.** When something asks you to pay (a spell, an
  ability, a "pay or …" tax), you first tap your lands as a plan, nothing
  happens yet, and then pay everything at once. A land tapped by mistake is
  simply taken back.
- **Reveals stand up.** When another player reveals a card, it is shown to
  you in the middle of the table until you put it away.
- **Easier to read at a glance.** Keywords that protect a card wear borders
  around it, each with its own colour and pattern (indestructible, hexproof,
  shroud, defender); a creature with summoning sickness shows a slow wave of
  moonlight; a phased-out permanent is drawn grey and faded.
- **Cards with unusual faces.** Adventures, flip cards, split and Aftermath
  cards and battles are shown the way they are read; holding <kbd>Shift</kbd>
  over a preview turns or rotates the card to its other half.
- **Language models hosted by the server.** A server operator can offer
  models of their own, and decide per player who may seat them.
- **For operators:** an admin console in the browser (accounts, bans,
  password resets, ending games, blocking cards, beta keys, hosted models,
  server load), per-gateway terms of use and a link to the operator's legal
  notice (Impressum).
- **An Intel Mac build.** Six downloads now: Windows (x64, ARM64), macOS
  (Apple silicon, Intel) and Linux (x86-64, ARM64).
- **500 more cards play exactly as printed** (2,702 in beta.6, 3,202
  now), most of them partial cards finished; partial cards went from 571 to
  68 ([Where the cards stand](#where-the-cards-stand)).

This release is for desktop computers only. The browser version and the
phone apps are not part of beta.7 ([docs/mobile.md](docs/mobile.md) says
where they stand).

## Screenshots

The screenshots show Baylee's **text view**, which draws each card from its
rules text instead of the printed card image ([License and legal](#license-and-legal)
explains why). In the game, the card images come from Scryfall, and <kbd>T</kbd>
switches between the two views. In the text view a card far off on the table
shows only its colour, its cost and its marks; pointing at it shows its text
large. The pictures were taken with 0.1.0-beta.7 on a local test server, with
card images switched off and made-up players.

<table>
  <tr>
    <td width="50%"><img src="docs/images/readme/lobby.webp" alt="The lobby with an open table, a table invite, the friends drawer and a private chat"><br><sub><b>The lobby, with friends and chat.</b> A friend invites you to her table, the friends drawer shows who is online and where, and the private chat is end-to-end encrypted.</sub></td>
    <td width="50%"><img src="docs/images/readme/table.webp" alt="A duel in progress with the chat window open on its All tab"><br><sub><b>At the table.</b> The chat window holds the game log as a tab; <i>All</i> mixes log lines and table chat, and every card name is a link.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/room.webp" alt="A room being arranged: one seat taken, an empty seat offering AI and Language model"><br><sub><b>Arranging a table.</b> Rules, teams and seats; an empty chair takes another player, the house AI or a language model.</sub></td>
    <td width="50%"><img src="docs/images/readme/payment.webp" alt="The payment window: a tax is owed and one land is staged"><br><sub><b>The payment window.</b> A tax is owed: the lands you click are staged, nothing taps until you press <i>Pay</i>, and <i>Untap all</i> takes them back.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/replay.webp" alt="A replay with its timeline, transport controls and the Watch from menu"><br><sub><b>Replays.</b> One cell per turn, play and pause, steps and speeds, and <i>Watch from…</i>: your seat, another player's, the table or all hands.</sub></td>
    <td width="50%"><img src="docs/images/readme/deck-builder.webp" alt="The deck builder with a search for walls and a 60-card deck grouped by type"><br><sub><b>The deck builder.</b> A search over the whole pool (here: playable walls), the deck grouped by type, and Import, Export and History.</sub></td>
  </tr>
  <tr>
    <td colspan="2"><img src="docs/images/readme/keyword-borders.webp" alt="Five creatures on the battlefield, each with its own border or shimmer"><br><sub><b>Read at a glance.</b> From the left: shroud (violet mist), indestructible (golden rim), summoning sickness (a pale wave of moonlight), defender and hexproof together (a stone wall inside a thin blue line), and summoning sickness again.</sub></td>
  </tr>
</table>

## Installing

Download Baylee for your system from the
**[releases page](https://github.com/AceVik/baylee/releases)** (pre-releases
are listed there too), or from the [website](https://baylee.acevik.de/),
which picks the right file for you. Each system has an **installer**, which
is the easy way, and an **archive** with the same game to unpack anywhere.
Baylee then updates itself ([Updating](#updating)).

| System | Installer | Archive |
| --- | --- | --- |
| Windows, Intel/AMD 64-bit | `Baylee-Setup-<version>-x64.exe` | `baylee-client-<version>-x86_64-pc-windows-msvc.zip` |
| Windows on ARM | `Baylee-Setup-<version>-arm64.exe` | `baylee-client-<version>-aarch64-pc-windows-msvc.zip` |
| macOS, Apple silicon (M1 and later) | `Baylee-<version>-aarch64.dmg` | `baylee-client-<version>-aarch64-apple-darwin.zip` |
| macOS, Intel | `Baylee-<version>-x86_64.dmg` | `baylee-client-<version>-x86_64-apple-darwin.zip` |
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

Apple silicon Macs (M1 and later) take the `aarch64` build, Intel Macs the
`x86_64` one (Apple menu → About This Mac names the chip or processor).
macOS 11 or later is required.

1. Open `Baylee-<version>-aarch64.dmg` (Intel: `-x86_64.dmg`) and drag
   **Baylee** onto **Applications**. Then eject the disk image. With the zip
   instead, unpack it and drag `Baylee.app` into **Applications** the same
   way.
2. Open Baylee from Applications. The first time, macOS refuses because the
   app is not notarised.
3. Open **System Settings → Privacy & Security**, scroll down to the message
   about Baylee and click **Open Anyway**.

Or, in Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Baylee.app
```

If you start Baylee straight from the download (the disk image, or the
unpacked folder in Downloads), macOS runs it from a hidden read-only copy.
Baylee still updates itself then, and its settings screen offers **Move to
my Applications folder**: it copies itself to `~/Applications` (no password
needed), checks the copy, starts it and closes. On its first start the copy
asks whether to move the old one to the Trash; nothing is deleted unless you
say so.

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

### Checking the download

Next to each installer and archive on the release page there is a `.sha256`
file, and the release notes list the installers' checksums too. Download the
file and its `.sha256` into the same folder and run:

```bash
# Linux
sha256sum -c Baylee-<version>-x86_64.AppImage.sha256
# macOS (an Intel Mac: Baylee-<version>-x86_64.dmg.sha256)
shasum -a 256 -c Baylee-<version>-aarch64.dmg.sha256
```

On Windows, run
`certutil -hashfile Baylee-Setup-<version>-x64.exe SHA256` in PowerShell
and compare the result with the number in the `.sha256` file.

### Updating

Baylee updates itself. It asks GitHub for a newer release at start and every
six hours, downloads it, checks its signature, and installs it when you
quit; the lobby says "Update X ready – installs when you quit", and the next
start says "Updated to X" once. An update whose signature does not verify is
never installed. The settings screen has two switches, "Update
automatically" and "Check for updates automatically" (off: Baylee asks
GitHub nothing until you press "Check for updates"); `docs/privacy.md` says
what GitHub sees.

Updates are kept in your own state folder (`~/.local/state/baylee`, or
`%LOCALAPPDATA%\baylee` on Windows), never written into the program's
folder, so the Windows setup's installation, a Mac app wherever it lies and
an AppImage in a folder of your own all update themselves. Where the
program's folder belongs to someone else (the `.deb`'s `/opt/baylee`, for
example), Baylee never asks for administrator rights: it links to the
release page, and you update by hand by installing the new package over the
old one. Releases up to 0.1.0-beta.5 did not update on a Mac while macOS ran
them from that read-only copy: move such a `Baylee.app` into Applications
once by hand.

**Where your things are.** Replacing the program keeps your settings,
because they are not stored next to it: on Windows in `%APPDATA%\Baylee`, on
Linux and macOS in `~/.config/baylee/` (`$XDG_CONFIG_HOME/baylee` if you set
that variable). There you find

- `client-settings.json`: language, your list of gateways, report consent,
  and a guest's session;
- `preferences.json`: key bindings and standing answers;
- `offline-decks.json`: decks built offline;
- `update.json`: the two update switches;
- `records/`: your last 20 games against the house, for replays
  (`records/archive/` keeps the ones you archived);
- `chat/`: your chat history, encrypted, and your device's chat key where
  the system has no keychain for it;
- `llm-seat.json` and `llm-spend.json`, once you set up a language model.

Card images are cached separately, in `~/Library/Caches/baylee` (macOS),
`%LOCALAPPDATA%\baylee` (Windows) or `~/.cache/baylee` (Linux). Your account,
your online decks, your friends and your replays live on the gateway, so
they are there on any computer you sign in on.

Every build shows its version and commit in the lobby. A new version may be
unable to talk to an older gateway, so update when a new release comes out.

## Your first game

When Baylee starts, you stand at its **front door**: a list of gateways
(servers) and the way to play offline. A short guided tour shows you around
the first time, and <kbd>?</kbd> shows every key that works right now, on
any screen.

**Offline, right away.** Choose **Play offline**. You get the lobby with
your offline decks, the deck builder, and **Play the house**, which starts a
game against the house AI at once. The small arrow on that button picks how
strongly the AI plays.

**Online.** Choose a gateway from the list (**Baylee Sanctuary** is already
in it, and you can add others by their address), then sign in:

- **Create account**: a *username* (3–24 letters, digits, `_ - .`), a
  *display name* and a password. No e-mail address is asked for. The
  username is private: other players only see your display name and a short
  tag, such as `Alice#af03`.
- **Play as guest**: a display name is enough. A guest can build decks and
  play, but cannot upload sleeves or playmats, and is deleted when it signs
  out or after 30 days without use.

You can delete your account from the client at any time; everything that
belongs to it goes with it.

A gateway may have **terms of use**: you are shown them when you create an
account, and again when they change. Where the operator names a legal notice
(Impressum), the client links it on the front door's *About*, beside the
terms and under *Settings → Network*.

### Getting in: closed-beta keys

Baylee Sanctuary is a closed beta: **a new account and a new guest each
need a key** of the form `BAYLEE-XXXX-XXXX-XXXX-XXXX`. Keys are handed out by
the project owner ([@AceVik](https://github.com/AceVik)); there is no public
sign-up form. Paste the key when you create the account or the guest;
upper and lower case, spaces and dashes do not matter. Once you have an
account, you sign in with username and password and need no key again.

## Playing with friends

**Decks.** In the deck builder you search the pool (if you like, only cards
that play exactly as printed), set main deck, sideboard and commander,
watch the mana curve, and pick a printing for each card. A deck can be
imported from and exported to text (Baylee's own, JSON, YAML and
Moxfield's text export; [docs/deck-format.md](docs/deck-format.md)). The
gateway keeps every version of an online deck, and a registered account can
give a deck its own sleeve and playmat. A card the gateway's operator has
blocked is marked, with the reason.

**Rooms.** In the lobby, open a table with 2 to 8 chairs and, if you like,
a name and a password. As host you decide who sits where: another player,
the house AI at one of its five levels, or a language model. You also set
the teams, the starting life, free mulligans and the clock (`casual`,
`standard`, `blitz` or `untimed`), and whether others may watch. Every
player picks their deck and says *Ready*, and the host starts. When the
game ends, the table can play again.

**At the table.** The game asks and you answer: the cards you can play
light up, and the bar at the bottom says what is being asked. With
teammates you can choose to show each other your hands. The most important
keys:

| Key | Does |
| --- | --- |
| <kbd>Space</kbd> | Confirm / pass priority / pay |
| <kbd>Enter</kbd> | Act on the card under the cursor, else pass |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>, <kbd>E</kbd> | Move the card cursor; play or select the card under it |
| <kbd>Esc</kbd> | Cancel (step by step), put away a revealed card |
| <kbd>Tab</kbd> / <kbd>⇧ Tab</kbd> | Fast-forward to the next phase / next turn (decisions stay yours) |
| <kbd>F6</kbd> / <kbd>F7</kbd> | Let the stack resolve / nothing more this turn |
| <kbd>K</kbd> / <kbd>B</kbd>, <kbd>Y</kbd> / <kbd>N</kbd> | Mulligan: keep / put a card on the bottom; yes / no |
| <kbd>T</kbd>, hold <kbd>Cmd</kbd>/<kbd>Alt</kbd> | Card text instead of the picture (switch / only while held) |
| <kbd>Shift</kbd> over a preview | Turn the card to its back or other half |
| <kbd>G</kbd>, <kbd>L</kbd> | Zone browser (graveyards, exile, stack), game log |
| <kbd>F8</kbd> | Report a problem |
| <kbd>?</kbd> | Every key that works right now |

Every key can be rebound in the settings, and a signed-in account keeps its
bindings on the gateway. [docs/keyboard-map.md](docs/keyboard-map.md) has
the complete list.

**Watching.** If a table allows it, anyone signed in to the gateway can
press *Watch* in the lobby and follow the game from the public view: no
hands, no hidden cards. The players see how many are watching.

**If your connection drops**, your chair waits for you: the client
reconnects by itself, and the other players see a countdown. If you are not
back in time, the house AI plays your chair until you return.

## Friends and chat

On a gateway with chat (Baylee Sanctuary has it), the **friends drawer** on
the right of the lobby lists your friends and whether they are online,
playing or watching. Add a friend by name and tag (`Alice#af03`), or from
the players you recently sat with. From a friend's row you can whisper to them,
watch their table or invite them to yours. Groups are chats for several
friends. At a table, everyone seated has a table chat, and every team its
own.

**What is encrypted.** Messages are **end-to-end encrypted**: they are
locked on the sender's computer for the computers of the people in the
conversation, and only those can open them. Here, honestly, is what that
means:

- **The server cannot read your messages.** It passes them on and keeps
  them, still encrypted, until they are delivered, for at most 14 days
  (the operator can choose 1 to 30; a guest's are kept for 1 day).
- **The server does see** who writes to whom and when, how long a message
  is, who is in a group and what the group is called, who is friends with
  whom, and who is online and at which table. It keeps no log of any of
  this.
- **Your devices are the weak point.** A message can be read on every
  device the people in the conversation use Baylee on, as long as those
  devices really are theirs. To be sure, compare **safety numbers** with a
  friend (the lock line at the top of the chat opens them). Anyone who can
  use your computer can read the chat history kept on it.
- **Table chat** goes only to the people seated at the table. Spectators,
  the house AI and language models never receive chat, and chat is never
  part of a game record or a replay.
- **Reporting a message** sends that one message, and nothing else of the
  conversation, to the gateway's moderators.

The chat window docks in the lower right of the lobby; at a table it is the
same window as the game log. Its settings (*Chat & privacy* in the friends
drawer) hold who may whisper to you, how long your history is kept, sounds,
and the list of your devices. [docs/privacy.md](docs/privacy.md) lists
exactly what the gateway keeps.

## Replays

The **Replays** tab in the lobby lists every game you can watch again:

- **Games against the house**, kept on your computer (the last 20, and any
  you archive).
- **Online games** you sat at, from the gateway, on any computer you sign
  in on.

A replay has its own controls at the bottom of the table: a timeline with
one cell per turn, play and pause, a moment or a turn back and forward,
start and end, and speeds from ½× to 8× (the keys: <kbd>Space</kbd>,
<kbd>←</kbd> <kbd>→</kbd>, <kbd>⇧ ←</kbd> <kbd>⇧ →</kbd>, <kbd>Home</kbd>
<kbd>End</kbd>, <kbd>-</kbd> <kbd>=</kbd>). The seat chip chooses **whose
view** you see: your own seat, the public table, every hand at once, or any
other player's seat.

**Who sees your hidden cards.** For each online game you decide whether the
other players may see your hand and library in that game's replays. For
games played from beta.7 on, sharing is **on** unless you switch it off
(the end-of-game screen shows it ticked, and the game's menu in the Replays
tab changes it at any time). For games played before beta.7 it is off,
because nobody was asked then. The house and language models always share.
A seat whose player said no shows a lock, and its hand stays hidden.

**Nothing is deleted.** *Archive* moves a game out of your list (and
*Unarchive* brings it back); the record stays on the gateway, because game
records are used to train the house AI and to balance decks
([docs/privacy.md](docs/privacy.md) says what a record holds: no chat, and
no names once an account is deleted). A replay is played again by today's
version of the engine; where that cannot be checked against what the table
showed back then, the timeline marks it.

## Playing against a language model

A chair can be played by a language model, such as one behind the
Anthropic or an OpenAI-compatible API, or a local model in LM Studio. It
plays from exactly what that seat would see, through the same questions a
person answers. No player's name is ever sent to a model: the seats are
called `P1`, `P2`, … and the model's own seat "you". The room shows which
model sits in which chair before anyone says *Ready*.

There are two ways.

### A model the server offers

Some gateways offer **hosted models**, which the server's operator runs and
pays for. The operator decides, for each model, which players may seat it.
If you may, an empty chair in your room offers **Hosted model**: pick one,
and it sits down. You need nothing on your computer.

### Your own model, on your own key

This works in the desktop client, on any gateway, and costs whatever your
provider charges.

1. **Set up a profile.** Open **Settings → Language models** and choose
   *Add an adapter*: Anthropic, OpenAI, DeepSeek, LM Studio on this
   computer, or an agent CLI you are signed in to (Claude Code, for
   example). A profile says which provider and model play and how hard the
   model thinks.
2. **Give it a key.** Paste the key into the key box and press *Keep*. The
   key goes into your system's keychain (macOS Keychain, Windows Credential
   Manager, the Secret Service on Linux), never into a file, and the panel
   never shows it again. A local model in LM Studio needs no key.
3. **Set caps.** Daily and monthly limits in dollars or tokens, plus a
   budget per game. Each game reserves its budget before it sits down and
   settles afterwards; the panel shows what you spent today and this month.
4. **Seat it.** In a room you host, press **Language model** on an empty
   chair, pick the profile, model and deck, and wait until the chair says
   *Ready*: the bridge first checks that the model answers.

How it gets in: your client asks the gateway for a short-lived **chair
ticket** for that chair and hands it to the seat bridge (`baylee-seat`,
which ships with the client). The model sits in the chair as *your*
delegate, named `LLM-…`, without an account of its own, so when you are
signed in with an account this works on a closed-beta gateway too. If the model fails, runs out of time or answers
something illegal, the house answers that one question for it.

**For tinkerers**, the same bridge runs from a terminal. Start a local
gateway and agent ([Running your own gateway](#running-your-own-gateway)),
then:

```bash
cargo run -p xtask -- dev-table --bridge profile:<name> --play   # you in chair 0, the model in chair 1
cargo run -p xtask -- dev-table --bridge house --play             # the house through the same bridge, as a check
```

The bridge writes what it asked and what the model answered to
`target/seat-transcripts/`. The settings file, the caps, the providers and
the CLIs are all described in [docs/llm-seat.md](docs/llm-seat.md).

## Reporting a problem

Press <kbd>F8</kbd> at the table or in the lobby, or use the report button in
the menu. A report goes to the gateway you are signed in to. It always
includes its kind, your text, the client version and, at a table, the game's
id.

Anything more has **one checkbox each**, and every box starts ticked:
untick what you would rather keep before you press *Send*. The boxes are
system and hardware, the table as you see it, your game log (with players
renamed to "Player A", "Player B", …), your settings, a picture of the
Baylee window only, and at a table the game's whole record (every move and
every seat's cards, no names). "Show what is sent" shows every byte. Session
tokens and chat are never sent. After a crash, the next start asks once
whether crash reports may be sent. If you are not signed in to any gateway,
the report goes straight to the feedback service the build names, and with
none named nothing is sent.

[docs/privacy.md](docs/privacy.md) lists everything the client, the gateway
and the feedback service keep. A card that plays wrongly is the most useful
report you can send; you can also
[open an issue on GitHub](https://github.com/AceVik/baylee/issues).

## Where the cards stand

Every card in Baylee is code: one file per card, written in a small rules
language, and played in a test. There is no interpreter guessing at card
text, which is why the pool grows batch by batch instead of all at once.

Counted on the beta.7 release candidate (13 October 2026):

| | Cards |
| --- | --- |
| Play **exactly as printed** (`Implemented`) | **3,202** |
| Play, but **not every sentence** yet (`Partial`, the deck builder warns) | 68 |
| In the pool, **no rules yet** (the deck builder warns) | 48 |
| **Never built** on purpose (ante, dexterity and subgame cards) | 13 |
| **All Magic cards** we are working towards (every distinct card, one per Oracle text) | 33,694 |

So about **one card in ten** of all of Magic plays today. The deck builder
can show only playable cards, and it warns when a deck holds cards that will
not play as printed.

**Sets.** Cards are built set by set in the order Magic printed them, each
card counted in the set it first appeared in:

| Set | Cards | Play as printed | Partial | No rules yet | Not in the pool yet | Never built |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Limited Edition Alpha (and the two cards Beta added) | 292 | 288 | 0 | 0 | 0 | 4 |
| Arabian Nights | 77 | 64 | 11 | 0 | 0 | 2 |
| Antiquities | 85 | 81 | 3 | 0 | 0 | 1 |
| Legends | 310 | 277 | 24 | 6 | 0 | 3 |
| The Dark | 119 | 19 | 0 | 2 | 98 | 0 |
| Fallen Empires | 102 | 23 | 0 | 0 | 79 | 0 |

**Alpha is complete**: every card that can be played plays exactly as
printed (the four left out are Chaos Orb, a dexterity card, and three ante
cards). Arabian Nights, Antiquities and Legends are in the pool in full;
their partial cards are being finished now. The Dark, Fallen Empires and
Ice Age come next. Cards from later sets that are already playable came in
with earlier batches.

**Known gaps.** No banlists or format-legality checks yet; the
variant formats' special objects (planes, schemes, vanguards, conspiracies,
dungeons) are not built. The [website](https://baylee.acevik.de/#cards)
shows every set and card with its status, and for a partial card what it
does not do yet;
[docs/mechanics-roadmap.md](docs/mechanics-roadmap.md) §E8 is the plan, and
[data/card-pool.txt](data/card-pool.txt) the list of cards in the pool.

## Running your own gateway

A gateway is the server players sign in to. You can run one for your own
group: everything the public one does, on your own machine. A game runs as
three kinds of process:

- **`baylee-gateway`** handles accounts, decks, rooms, friends, chat and
  replays. It runs no rules itself and needs PostgreSQL.
- **`baylee-agent`** starts one engine process per game when the gateway
  asks for one.
- **`baylee-engine-server`** is one game. It dials back to the gateway.

This section is a guide. The normative references are
[docs/protocol.md](docs/protocol.md) and the *Environment* section of
[docs/dev/environment.md](docs/dev/environment.md).

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
| `BAYLEE_SOCIAL` / `BAYLEE_SOCIAL_KEEP_DAYS` | Friends and chat (`off` turns them off); how many days an undelivered message is held (1–30, default 14). |
| `BAYLEE_REPLAY_JOBS` | Replays computed at once (default 4; `0` turns replays off). |
| `BAYLEE_TERMS_PATH` | Your terms of use: one Markdown file, or a folder of `terms.<lang>.md` files with `terms.en.md` among them. Players accept them at sign-up and after a change. |
| `BAYLEE_IMPRESSUM_URL` | A link to your legal notice (Impressum), which clients show. |
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
set up with `baylee-deploy` (crate baylee-ops), the wrapper
`sudo baylee-invite …` reads the gateway's settings for you. The admin
console below does the same in the browser.

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
it. This minimal [Caddy](https://caddyserver.com/) example is a starting
point (`scripts/server/` has the ones the public server uses):

```caddyfile
baylee.example.org {
    reverse_proxy 127.0.0.1:28766
}
```

Caddy obtains a certificate, passes WebSockets through and sets
`X-Forwarded-For`. Then set `BAYLEE_TRUSTED_PROXIES=127.0.0.1` on the
gateway. The gateway authenticates with bearer tokens and sets no cookies,
so no further CORS or cookie configuration is needed.

### 7. Bug reports and the admin console (optional)

`baylee-feedback` is a separate service that collects the reports gateways
forward, with a web interface for reading them. The same interface is the
**admin console** for one gateway, for admins you create with
`baylee-feedback admin add`: an overview with the server's load, who is
online and at which table, accounts (reset a password, ban, delete), ending
a running game, blocking a card from play for a while or for good (players
are told why), beta keys, and the hosted language models (which seat agent
runs which model, their keys, and which players may seat each). The
gateway's side of it listens only on the local machine
(`BAYLEE_ADMIN_TOKEN`, `BAYLEE_ADMIN_BIND`). Setting it up is described in
[docs/feedback.md § Running it](docs/feedback.md#running-it); without it,
the gateway refuses reports with "reports are not configured".

### 8. Hosted language models (optional)

To offer models to your players, run a **seat agent** (`baylee-seathost`)
beside the gateway, with `BAYLEE_SEATHOST_TOKEN` set on both. Its models,
their keys and their spending caps live on the seat agent's machine; the
admin console sets them up and decides who may seat which model.
[docs/llm-seat.md § A hosted seat](docs/llm-seat.md#a-hosted-seat) and
[docs/protocol.md](docs/protocol.md) §"Hosted language-model seats" have
the details.

## Building from source

You need **Rust stable, 1.95 or newer** (`rust-toolchain.toml` pins the stable
channel with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target). On
Linux you also need `libasound2-dev libudev-dev libwayland-dev`.

```bash
git clone https://github.com/AceVik/baylee.git
cd baylee
cargo run --release -p baylee-client          # the game; offline play needs nothing else
```

- **A local table with seats:** start a gateway and an agent (above), then
  run `cargo run -p xtask -- dev-table --seats 4 --ai sharp --play`.
- **The browser and phone builds** still compile
  (`trunk serve index.html --release` in `crates/baylee-client/`;
  `cargo dev android`, `cargo dev ios-sim`), but they are not part of this release
  ([docs/mobile.md](docs/mobile.md)).

For contributors, the checks CI runs:

```bash
cargo fmt --all
cargo dev gate rules                        # quick: rules crates, without the client
DATABASE_URL=… cargo dev gate               # the full gate: fmt, clippy, nextest, validate
DATABASE_URL=… cargo dev gate features      # the non-default feature builds
```

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
  seal[baylee-seal] --> clientcore
  clientcore --> client[baylee-client<br/>Bevy]
  clientcore --> seat[baylee-seat]
  protocol --> gateway[baylee-gateway]
  protocol --> agent[baylee-agent]
  db[baylee-db] --> gateway
  catalog[baylee-catalog] --> gateway
  seal --> gateway
```

Each arrow drops a capability. The **engine** is synchronous and
deterministic: no I/O, no clock, a seeded random-number generator, so the
same game with the same answers always ends the same way, which is what
makes records and replays possible. **gamehost** builds each seat's view
from it, and that view hides what the seat may not see. **client-core** is
the client's logic without a renderer and holds most of the client's tests;
the 3D table is built with [Bevy](https://bevyengine.org/). **seal** is the
chat's encryption. The **gateway** and the **agent** link no rules at all.
Cards are Rust code, written in a small DSL, one file per card under
`crates/baylee-cards/src/cards/`.

Where to read on:

| Topic | Document |
| --- | --- |
| Overall plan | [docs/architecture.md](docs/architecture.md) |
| Engine internals (layers, events, cleanup, loops) | [docs/engine-internals.md](docs/engine-internals.md) |
| Wire protocol, gateway, rooms, chat, replays, keys | [docs/protocol.md](docs/protocol.md) |
| The client | [docs/client.md](docs/client.md) |
| Writing cards | [docs/card-dsl.md](docs/card-dsl.md), [docs/card-identity.md](docs/card-identity.md) |
| House AI | [docs/house-ai.md](docs/house-ai.md) |
| Language-model seats | [docs/llm-seat.md](docs/llm-seat.md) |
| Privacy | [docs/privacy.md](docs/privacy.md) |
| Legal | [docs/legal.md](docs/legal.md) |
| Releasing | [docs/releasing.md](docs/releasing.md) |

## Contributing

Issues and pull requests are welcome on
[GitHub](https://github.com/AceVik/baylee), and the
[Discord](https://discord.gg/4pP3uAwgY) is the place to talk and find
opponents. Before you start on code, read [AGENTS.md](AGENTS.md): it is the
map of the project's working rules, for people and coding agents alike, and
names the document for each area. A few points:

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

**Additional terms (AGPL section 7).** [NOTICE](NOTICE) adds three terms
the licence allows: keep the notices, mark a modified version as modified
and do not pass it off as the official Baylee, and no rights in the names
"Baylee" and "Baylee Central" or the logo.

**Accounts.** Sign-in goes through Baylee Central, the project's account
service ([AceVik/baylee-central](https://github.com/AceVik/baylee-central)).
Its source is public so that anyone can check how it handles their data;
its server code is source-available, while the client libraries Baylee links
(`central-api`, `central-client`, `central-crypto`) are AGPL like Baylee.
Central's terms of use admit only the official clients. Default terms of use
and a privacy notice template for anyone running a gateway are in
[docs/gateway-legal](docs/gateway-legal/README.md).

**Fan content.** Baylee is unofficial Fan Content permitted under the
[Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy).
Not approved/endorsed by Wizards. Portions of the materials used are property
of Wizards of the Coast. ©Wizards of the Coast LLC. Magic: The Gathering,
card names, set names, card images and mana symbols are property of Wizards
of the Coast LLC. Baylee is not affiliated with Wizards of the Coast, and no
Wizards of the Coast asset is shipped in this repository.

**Scryfall.** Card data and images provided by
[Scryfall](https://scryfall.com). The client loads card images from Scryfall
while you play; they are not part of this repository and are not
redistributed. For that reason the screenshots above use Baylee's own text
view, which draws a card from its name and rules text in our own layout, with
no frame art or image of a printed card
([docs/legal.md](docs/legal.md), clauses 2 and 3). Baylee is not affiliated
with Scryfall.

**Third-party material.** [NOTICE](NOTICE) lists the bundled fonts (Alegreya
Sans, Faustina, Font Awesome Free and the Mana font, all SIL OFL 1.1);
[docs/legal.md](docs/legal.md) §5 covers the CC0 instrument recordings the
music is played with. NOTICE also names the external, GPL-licensed
card-script corpus that code generation may read from a developer's own
checkout. No file of that corpus is ever copied into this repository or
shipped with a build.
