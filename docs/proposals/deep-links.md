# Deep links from baylee.app into the client

Design proposal, 22.10.2026. **Slice 1 is built** (30.10.2026, owner's go:
"Make a good handling of if the user is authenticated or not"); its
normative description is `docs/client.md` §"Links from the website", and
two findings of building it correct this text: macOS delivers the link to
the client, not the launcher, so the launcher changed only for Windows and
Linux; and winit 0.30 owns the macOS app delegate, so the client asks
`NSAppleEventManager` for the link. Slices 2 and 3 are not built. On the same
day the owner chose how signing in from the website works (pairing with the
master key, a key check, a vault key from the master key, rotation): the
section "Protocol specification" below is that decision, and it replaces
the options A, B and C and questions 2 to 4. Its parts are named so Central's own design pass can take them up;
Central work runs at the highest effort, and a new Central dependency needs the
owner's approval. The questions the owner has to decide close this document.

The owner's wish (22.10.2026, translated):

> When you are signed in on the website, you should be able to start the
> client via a deep link; you are then signed in directly and may choose the
> gateway. Via a deep link on the website's gateway list you can also add
> gateways: the deep link starts the client too (if it is not running yet,
> also signed in), and the new gateway then shows up in the gateway selection.

## In short

- A link can start the client, bring it forward and propose a gateway without
  carrying anything secret or personal. That part needs no Central change.
- "Signed in directly" is the hard part. The account's master key opens the
  account document, and the document holds the gateway list the person wants
  to choose from. Only the password (or the recovery code) unwraps that key.
  The website keeps nothing after its own sign-in. So a link alone cannot sign
  the client in with full access. Either the person types the password (in
  the client, or once more on the website), or the website keeps the master
  key while signed in. Then a pairing hands the key to the client, sealed to a
  key the client made for it.
- A sign-in that gives the client a Central session but no master key is
  possible on the wire, but it is not worth building. The client could not
  read the account's gateways, preferences or chat archive.
- Every second launch goes through the permanent launcher, which refuses it
  today. Handing a link to the running client means changing the launcher.
  That costs every player one manual reinstall.
- Recommended order: slice 1 (the scheme, the launcher handoff, "open" and
  "add gateway" with a confirmation), slice 2 (the website's buttons), then
  slice 3 (pairing) only after the owner's answers.

## What exists today

| Area | Where | What it means for links |
| --- | --- | --- |
| macOS bundle | `crates/baylee-tools/src/package.rs:200` (`info_plist`) | No `CFBundleURLTypes`. `CFBundleExecutable` is the launcher. |
| Windows setup | `scripts/installers/windows/baylee.iss` | Per user, HKCU, no admin rights. No URL protocol keys. |
| Linux entry | `scripts/installers/linux/baylee.desktop` | `Exec=baylee`, no `%u`, no `MimeType`. Used by the `.deb` and the AppImage (`package.rs:490`, `:555`). |
| Installer checks | `crates/baylee-tools/src/release/check.rs:385` | `desktop-file-validate` already runs on the `.deb`'s entry. |
| Launcher | `crates/baylee-update/src/bin/baylee-launch.rs:15` | Passes its arguments on to `launch::run`. |
| Second launch | `crates/baylee-update/src/launch.rs:574`–`576` | Takes `entry.lock`, then `lifetime.try_lock()`, before it looks at any argument. A second launch fails with "another instance may be running". |
| Launcher lifetime | `docs/releasing.md` §"Desktop launcher and recovery" | "Permanent in every package"; "a launcher protocol change likewise requires a new manual package". |
| Secret handoff | `crates/baylee-update/src/relaunch.rs` (module doc) | A sign-in crosses processes only on a pipe: "never in an argument, the environment, a file or a log". |
| winit 0.30.13 | `src/platform/macos.rs` (module doc) | No opened-URL event on any platform. On macOS winit "will not register an application delegate", so the client may set one. |
| Typed gateway address | `crates/baylee-client/src/lobby/gateway.rs:8` (`normalize`), `:137` (`check_gateway`), `:153` (`gateway_answered`) | Checks the address, probes it, saves it only when a gateway answered. |
| Probe | `crates/baylee-client/src/lobby/http.rs:897` (`probe_gateway`) | `GET /info`, then `GET /auth/config`. |
| Central sign-in | `crates/baylee-client/src/lobby/central.rs:176` (`enable`), `:252` (`login`), `:306` (`receive`), `:431` (`sync_gateways`) | The restored document replaces the gateway list; edits go back through `Account::save`. |
| Account worker | `crates/baylee-account/src/account.rs:27` (`login`), `:235` (`save`); `data.rs:62` | `login` takes the password. `save` is compare-and-swap. A document holds at most 128 gateways. |
| Local chat vault | `crates/baylee-client/src/social/vault_access.rs:31`, `docs/password-vault.md` | The password, kept for 120 s after a Central sign-in, also opens the device's own chat vault. |
| Website session | baylee-central `docs/web-accounts.md` | Cookie `__Host-central`, `kind = browser`. "The page unwraps the master key only to check the password worked and keeps nothing." A browser session is never a native bearer. |
| Browser operations | baylee-central `crates/central-identity/src/lib.rs:70` (`browser_operation`) | One exhaustive list. A new operation does not compile until it is decided there. |
| Website gateway list | baylee-central `web/src/gateways/GatewayCard.tsx`, `web/src/central/directory.ts:98` | "Copy address" only. The directory is read through `/api/v1/native`, which the browser boundary refuses today. |
| Central's plan | baylee-central `docs/roadmap-accounts-web.md` (§"Target shape", M6); `docs/architecture.md` | "The installers register `baylee://`"; sign-in by authorization code with PKCE (RFC 8252, RFC 7636); "An authorized unlocked device can recover through explicit, authenticated pairing." |
| Phones | `docs/mobile.md`; `.github/workflows/client-packages.yml` | Android is a development APK with `NativeActivity`; iOS runs in the simulator only. Neither has a release row. |

## The links

One scheme, three links, and one more for slice 3. This document writes the
scheme as `baylee:`, as Central's roadmap does. Question 6 asks whether it
should be `app.baylee:` instead.

| Link | Means | Carries |
| --- | --- | --- |
| `baylee://open` | Start the client, or bring it forward. | Nothing. |
| `baylee://signin` | As `open`, then show sign-in at Central. With slice 3, start a pairing. | Nothing. |
| `baylee://gateway/add?origin=<origin>` | Propose a gateway. | One public origin, percent-encoded. |
| `baylee://signin/done?code=<code>` | The end of a pairing (slice 3 only). | A one-use code, worthless without the client's verifier. |

The parser is strict:

- At most 2 KiB, ASCII only after percent-decoding the origin.
- Host and path exactly as above, the named parameters exactly once each.
  Anything else, and the whole link is ignored.
- A link never carries text the client shows. The client builds every word of
  its dialogs itself.
- The client contacts no address named by a link before the player confirms.
- The log names only a link's kind ("a gateway link"), never its text.

The client does not log its arguments today: `console.rs`, `lobby/resume.rs`
and `update/native.rs` read them and print none. That has to stay so, and
dev-control must not echo a link either.

## Registering the scheme, per platform

The scheme belongs to the original package. Updates never change that package
(`docs/releasing.md` §"Desktop launcher and recovery"). So every player gets the
links only by installing a new package by hand once. The same package brings
the new launcher (below), so both arrive together.

### macOS

- `info_plist` gains `CFBundleURLTypes` with `CFBundleURLSchemes` = `baylee`.
  `package-client.sh` signs after writing it, so the ad hoc seal holds.
- macOS delivers an opened URL as an Apple Event (`kAEGetURL`, seen by an
  app delegate as `application:openURLs:`), not as an argument. Today the
  launcher is the bundle's executable and has no run loop. The client runs as
  its child, with its own `NSApplication` from winit, often from another copy
  of the bundle in `versions/<id>/Baylee.app`. Which of the two processes gets
  the event, on a cold start and while the client runs, is not known.
  **A spike decides it** with the real packaged app, started from Finder.
- A second risk for that spike: every downloaded generation is a whole
  `Baylee.app` with the same `Info.plist`. LaunchServices may register those
  copies as handlers of the scheme and prefer a newer one. A link would then
  start the launcher inside the state directory, which keys a separate
  installation ("Moving or renaming the original package starts a separate
  state directory"). If the spike sees that, the launcher has to notice it runs
  from its own state root and hand over to the original package.
- Native code, in any case: an `NSApplicationDelegate` in the client with
  `application:openURLs:`, set once winit's event loop exists and before it
  runs. `bevy_winit` 0.20 builds the loop in `WinitPlugin::build`, so a later
  plugin's `finish` is the place, before `App::run`. `objc2` 0.6,
  `objc2-app-kit` 0.3 and `objc2-foundation` 0.3 are in `Cargo.lock` already
  and would become direct macOS dependencies of `baylee-client`. If the spike
  shows the launcher gets the event, the launcher needs a small AppKit run
  loop as well, which makes `baylee-update` depend on the same crates.

### Windows

- The setup writes, under `HKCU\Software\Classes\baylee`: `(Default)` =
  `URL:Baylee`, `URL Protocol` = empty, `DefaultIcon`, and
  `shell\open\command` = `"{app}\baylee-client.exe" "%1"`, with
  `uninsdeletekey`. Per user, no administrator rights, as the rest of the
  setup.
- Cold start works with today's launcher code: Windows puts the link in the
  launcher's arguments, and `launch::run` passes them on to the client.
- A running client: Windows starts a second launcher, which fails at the
  lifetime lock. It has to hand the link over instead (below).
- Windows lets a background process take the foreground only when the
  foreground process allows it. The second launcher was started in the
  foreground by the browser, so it can allow the client
  (`AllowSetForegroundWindow`) before it exits. To be checked.
- `windows-sys` 0.61 is a dependency of `baylee-client` already; the pipe
  needs a few more of its features.

### Linux

- `baylee.desktop` gains `MimeType=x-scheme-handler/baylee;`, and `Exec`
  becomes `baylee %u`.
- The `.deb` should need no maintainer script: where `desktop-file-utils` is
  installed (on the common desktops it is), its dpkg trigger on
  `/usr/share/applications` refreshes the MIME cache. To be checked on the
  runner: `check-installers` should assert
  `xdg-mime query default x-scheme-handler/baylee` after `apt` installs it.
- An AppImage is registered only when something integrates it (appimaged,
  AppImageLauncher, Gear Lever), as with its icon today.
- On Wayland, a second process can raise the running window only with the
  activation token the desktop handed it (`XDG_ACTIVATION_TOKEN`). The
  launcher forwards it. Whether winit 0.30 can apply it to a window that
  already exists is to be checked. Focus is not promised.

### Android and iOS

Neither ships today, so neither is in scope.

- Android: cargo-apk writes an intent filter into the manifest. Whether
  `android-activity` 0.6 with `NativeActivity` hands a running activity's new
  intent to native code is to be checked; it may need a Java activity.
- iOS: winit owns UIKit's application delegate. Out of scope.
- On phones any app may claim a custom scheme. There, verified https links
  are the safer road: Android App Links (`/.well-known/assetlinks.json` on
  baylee.app) and iOS Universal Links. The latter need a signed app, that is
  the Apple Developer membership `docs/releasing.md` §"Installers" prices.

## One client: the link that arrives while it runs

### The handoff, in the launcher

`launch::run` changes in one place. When `lifetime.try_lock()` fails and the
arguments hold a `baylee:` link, the launcher connects to the running
client's channel, writes the link, waits for one byte of answer and exits 0.
Without a link it keeps today's refusal. (Bringing the running client forward
on a plain second start would be a nice extra.)

The channel is frozen, because the launcher is permanent:

- One connection, one frame: a magic and version byte, a 16-bit length, the
  link (at most 2 KiB). The client answers one byte, taken or refused, and
  closes. On Linux the frame also carries the activation token.
- Bytes on a pipe or socket only. Never a file, never the client's
  arguments, never the environment. A pairing code is in there (slice 3).
- One channel per installation, named after the state key the launcher
  already computes. Windows: a named pipe the client creates with
  `FILE_FLAG_FIRST_PIPE_INSTANCE` and an access list for its own user only.
  Linux and macOS: a Unix socket in a `0700` directory in the installation's
  state directory, removed when the client exits.
- Two installations have two state keys, so two channels. The link reaches
  whichever package the system registered last.
- During "Restart now" the old launcher still holds the lock and no client
  listens. A link that arrives then is lost; the launcher says so on stderr.

The operating system itself puts a cold-start link into the launcher's
arguments, which other processes of the same user can read. That is why a
link never carries a secret, and why the pairing code is worthless alone.

### What the client does with a link, by state

Nothing a link asks interrupts a game, and nothing is entered by itself.

| State | `open`, `signin` | `gateway/add` |
| --- | --- | --- |
| Not running | Starts. The front door as usual. | Starts. The proposal waits until the front door stands. |
| Front door, signed out (the device's gateways) | Comes forward. `signin` opens Central's form. | The confirmation, over the list. "Add" saves to the device's list (the settings file). |
| Central's form open, not yet restored | Comes forward. | Held: the device's gateways are hidden then (`central.rs:550`). After the restore it is offered for the account's list. Back with `‹`, it is offered for the device's list. |
| Signed in at Central, the account's gateways shown | Comes forward. | The confirmation. "Add" goes through `sync_gateways` and `Account::save`. |
| Inside a gateway: hub, room, deck builder, settings | Comes forward. | A quiet notice on the hub. The confirmation opens from the notice or on the gateway list, never over a room or a builder with a draft. |
| A game or a replay (`DuelPhase` not `Closed`) | Nothing is shown and no sound plays. | Held until the player is back, as `RestartOffer` waits (`client-core/src/resume.rs:284`). |

- One proposal waits at a time. A newer link replaces it while no dialog is
  open; while one is open, a newer link is dropped.
- At most ten links a minute are read; the rest are dropped without a word.
- The added gateway is never selected or entered by itself.
- `signin` brings up Central's form through `central::enable` without a bound
  gateway. Today that state has no way back
  (`central_turned_on_without_a_gateway_has_no_way_back`, `central.rs:652`);
  opened from a link, it needs `‹` back to the device's gateways.
- Already signed in at Central, `signin` only brings the client forward.

## Signing in from the website

### What the website holds when the person clicks

- The website signs in with OPAQUE in the page. The master key is wrapped
  under the OPAQUE export key. The page unwraps it only to check the password
  and keeps nothing (baylee-central `docs/web-accounts.md`).
- The session is a cookie of `kind = browser`. Central stores it under its own
  digest, so it can never act as a native bearer. The server PR made that a
  deliberate fix: before it, "a stolen cookie would have been a full native
  session".

- Sessions renew (owner, 22.10.2026, being built in Central): native and
  browser alike, about 30 more minutes per renewal, at most 12 hours after
  the password sign-in. Before that a session ended after 30 minutes. So a
  person may now stay signed in on the website for a working day, and the
  click may come hours after the password was typed.

So at the moment of the click, the website holds nothing that gives full
access.

### What the client needs for full access

`Account::login` (`account.rs:27`) runs `central_client::login` with the
password. That gives a native session and the master key. With the key the
client reads and checks the account document (gateways, preferences, chat
archive keys), confirms the restore and buys gateway tickets. Central's
`docs/native-login.md`: "An enrolled account document is mandatory." The
password also opens the device's own chat vault for 120 seconds
(`vault_access::Login::central`); no master key opens that one.

**A handoff without the password cannot give the client the same access,
unless something hands over the master key.** The options:

**A. Open the client and sign in there.** `baylee://signin` brings up
Central's form; the person types the password. Full access, no Central
change, no secret anywhere. This is slice 1.

**B. A session without the key.** Central could mint a one-use code for the
browser session, which the client redeems with a verifier for a native
session. The client could then not read the document: no gateway list (the
very thing the person wants to choose from), no preferences, no chat archive.
The whole sign-in path assumes the document. And it reverses the server's
decision that a browser session never becomes a native one. **Not
recommended.**

**C. Pairing.** The client makes a one-time key pair and a verifier. The page
seals the master key to the client's public key, and Central hands the sealed
key and a native session to the client that proves the verifier. This is the
"explicit, authenticated pairing" of Central's `docs/architecture.md`, with the
signed-in page as the unlocked device. The page has the master key at that
moment only in one of two ways:

- **C1.** The page asks for the password again ("Sign in Baylee on this
  computer") and runs a fresh browser sign-in. The person types the password
  once anyway, but in the browser, where a password manager fills it.
- **C2.** The page keeps the master key in memory for its session. That is a
  change of `web-accounts.md` ("keeps nothing"). Any script that ever runs on
  baylee.app could read the key then, and with renewing sessions "then" is up
  to 12 hours, not 30 minutes. The site's CSP makes that less likely; it does
  not rule it out. It is one click, but only until the tab reloads: a key in
  memory is gone after a reload, while the cookie session lives on. After a
  reload C2 falls back to C1. Keeping the key across reloads (in
  `sessionStorage` or IndexedDB) would put it where every script on the
  origin reads it, and is not proposed.

A fourth road, the client keeping the master key in the system's key store
between launches, is a design of its own and not proposed here.

### The pairing, step by step (C)

The client has to start it. A link from the website cannot carry the client's
key, and the master key cannot be sealed to a key the client has not made yet.
A link carrying a key would be a secret in a URL. So the website's button only
starts the client, and the client starts the pairing. That is the shape of
RFC 8252 (a native app signs in through the system browser) with PKCE
(RFC 7636), which Central's roadmap names.

1. **Website.** "Play in Baylee" opens `baylee://signin`.
2. **Client.** Makes a verifier `V` (32 random bytes), its challenge
   `C = SHA-256(V)` and an X25519 key pair. Asks Central
   `HandoffStart { challenge, public_key }` and gets `{ handle, expires_at }`
   (128 random bits, five minutes). Opens the system browser at
   `https://baylee.app/pair#<handle>`; a fragment is never sent to a server.
   Shows "Continue in your browser" and a comparison code: six Crockford
   characters of a SHA-256 over a fixed label, the handle and the public key.
3. **Website.** Asks `HandoffShow { handle }` and gets the public key. Shows
   "Sign in Baylee on this computer as `<name>`? Your Baylee shows 4KQ-7MX.",
   and that this ends Baylee's sign-in on any other device. With C1 it asks
   the password and signs in afresh. It seals the master key to the public
   key, binding issuer, account, epoch and handle, and sends
   `HandoffApprove { handle, sealed }` with the cookie. Central checks the
   handle is live and not yet approved, keeps `{ account, sealed }` beside the
   challenge, and answers a one-use code `K` (128 bits, 60 seconds) to the
   page only.
4. **Website.** Navigates to `baylee://signin/done?code=K`. This is the only
   place a code stands in a URL.
5. **Client.** The link reaches it through the launcher's handoff. It matches
   `K` to its pending pairing and sends `HandoffRedeem { handle, code,
   verifier }`. Central checks `SHA-256(V) = C`, the code, the expiry and that
   neither was used. It makes a native session, which ends any other native
   session of the account as a sign-in does, and answers `{ identity,
   session, sealed, expires_at }`. The client opens the sealed key, then reads
   the document and confirms the restore as after a sign-in. The new session
   renews like any other, but its 12 hours count from the password proof that
   unlocked the key: with C1 the fresh sign-in of step 3, with C2 the website
   session's own sign-in. A pairing never restarts the clock, so pairing
   again and again cannot keep an account open without the password.
6. The client forgets its private key, `V` and `K`. The page says "Done, back
   to Baylee".

Why the code comes back through a link: the link lands on the computer where
the browser runs. If someone else started a pairing and sent the person the
pair page, the person's approval sends `K` to the person's own computer,
where no pending pairing matches it, and the stranger never gets it. Another
app that claimed the scheme on the same computer gets `K` but not `V`. If the
client instead polled Central with the handle and `V`, a careless approval
would hand the account to whoever started the pairing: the "remote phishing"
of RFC 8628 §5.4. Then only the comparison code would stand in the way, and
people skip it.

What Central would need (for its own design and its three assurance passes):

- Four requests. `HandoffStart` and `HandoffRedeem` on the native surface;
  `HandoffShow` and `HandoffApprove` on the browser surface, each entered in
  `browser_operation` and in the per-address budget table of
  `web-accounts.md`. The native surface has only global budgets, so the start
  needs a cap on live handles as well. At most three redeem attempts per
  handle, then it is dead.
- A table: handle digest, challenge, public key, account, sealed key, code
  digest, expiry, used, and the time of the password sign-in the new native
  session inherits as the start of its 12 hours. Rows live five minutes.
- **The cookie alone must not approve.** Central cannot check that the
  sealed bytes hold the real master key. Someone who stole the cookie could
  start a pairing on their own machine, approve it with bytes that are not
  the key, and receive a native session: option B through the back door,
  and the very cookie-to-native step the server PR closed. With renewing
  sessions such a cookie is good for up to 12 hours. So `HandoffApprove`
  needs a password proof Central can check and recent enough: a browser
  sign-in finished within a few minutes (C1 always qualifies; C2 only
  shortly after the website's own sign-in, and asks again after that).
- Sealing: HPKE (RFC 9180, base mode, X25519, HKDF-SHA256,
  ChaCha20-Poly1305) or an equivalent sealed box. Central's graph has neither
  `hpke` nor `x25519-dalek` (Baylee's has `x25519-dalek` for `baylee-seal`). A
  new dependency, so the owner's approval.
- `central-web-crypto`: a sealing export, and with C2 a held master-key state
  that is wiped at sign-out and at the session's end.
- `central-client`: `handoff_start` and `handoff_redeem`, the second returning
  a session as `login` does. Today the only session comes from `login`, which
  takes the password.
- The web app: the `/pair` page and the "Play in Baylee" button.
- The privacy notice: the new record and its five minutes.

What Baylee would need:

- `baylee-account`: `Account::from_pairing`, which is `Account::login` after
  the OPAQUE step (read and check the document, confirm the restore). It
  depends on the SDK change above and a new Central pin.
- The lobby: "Sign in with baylee.app" on Central's form, the pending pairing,
  and the `signin/done` link.
- No `vault_access::Login::central` after a pairing: the device's own chat
  vault is imported at the next sign-in with the password. A legacy player
  who has not moved to Central cannot sign in on the website at all, so the
  legacy migration (which needs the password) is not affected.

## Protocol specification: pairing, key check, vault key, rotation

The owner's decision of 30.10.2026 ("Setzen wir das so um. Gerne mit
deinen Optimierungen und Verbesserungen"), relayed by the PM, replaces the
options A/B/C above. This section is the contract between the pieces built
in parallel: Central's server and crypto and Baylee's client and launcher
(this repository's sessions), and the website (the session "Central
website: redesign, account area, gateway tokens"). A change to a shape here
changes this section first. Central's own documents (`web-accounts.md`,
`native-login.md`, `crypto-protocol.md`) take the parts that are Central's
when they are built.

### The rule

The password unwraps the master key once, in the client or the page that
typed it, and is dropped at once. It is never stored, never handed to
another part of the program, never sent anywhere (OPAQUE sends no password).
Everything after that runs on the master key. A stolen master key harms
Baylee; a stolen password harms every site where it is used again.

### Derivations from the master key

All in `central-crypto`, as the object keys already are (`vault/object.rs`):
HKDF-SHA-512 with the master key as input, a fixed salt per purpose, and
the account context as info (`Context::bound`: the domain, then the 32-byte
issuer, the 16-byte account and the epoch as eight big-endian bytes). The
master key still has no export API; each derivation is a method that
answers its own key or value.

| Purpose | Salt | Info after the context | Output |
| --- | --- | --- | --- |
| Key check (`K`) | `baylee-central/key-check/v1` | nothing | 32 bytes, stored by Central |
| Local chat vault key | `baylee-central/local-vault/v1` | the vault's scope (`docs/password-vault.md`) | 32 bytes, never stored |

`K` is one-way: knowing it does not give the master key. It changes with
the epoch.

### Key check (owner's item 3)

Central stores `K` beside the account (`accounts.key_check`, 32 bytes,
nullable until set). It is set:

- at registration (`EnrollmentFinish` gains `key_check`);
- by a new master key: rotation (below), and an administrator's reset that
  makes a new vault (`ResetVault::New` gains `key_check`; a reset that
  keeps the vault keeps `K`, a password change keeps it too);
- for an account that has none: `SetKeyCheck { check }`, on either surface,
  accepted once, only while `key_check` is null and only within two minutes
  of a password proof (the session's `proven_at`, below). A stolen cookie
  can therefore never set it to the thief's value. The client and the page
  ask after every password sign-in (`KeyCheckStatus` → `KeyCheck { set }`,
  a new request on both surfaces) and send it when none is set.

New request fields carry `#[serde(default)]`, so an older client's request
still reads; no existing answer gains a field (the native `SessionStatus`
and `Session` stay as they are: clients built before refuse unknown fields).
A request that comes without `key_check` where it now belongs (an older
client's reset to a new vault) clears `K`, and the next password sign-in
sets it.

`K` itself never travels after that. Approving a pairing proves the master
key with a MAC under `K` over that approval (below); Central recomputes it
with the stored `K`. A thief with the cookie alone cannot make it. Without
`K` stored, no pairing can be approved; the page says so and the next
password sign-in sets it.

### Pairing, browser → client (owner's item 2)

Three parties: the page (signed in, master key held in its isolated worker),
Central, the client. Two secrets: `S`, 32 random bytes the page makes, which
only the page and the link carry (Central never sees it); and the client's
verifier `v`, 32 random bytes, which only the client holds.

| Step | Who | Message | What is checked |
| --- | --- | --- | --- |
| 1 | page → Central | `PairStart` (browser surface, cookie) → `PairStarted { handle, expires_at }` | a live browser session; the account has `K`; at most 3 open pairings per account |
| 2 | page → client | opens `baylee://pair?h=<handle>&s=<S>` | — |
| 3 | client → Central | `PairClaim { handle, public_key, challenge, mac }` (native, no session) → `PairAccepted { expires_at }` | the handle is open and unclaimed, its claim window (2 minutes) not over; exactly one claim per handle, a second is `Conflict` |
| 4 | page → Central | `PairStatus { handle }` → `PairClaimed { public_key, challenge, mac }`, or `Pending` | the session that started it |
| 5 | page | checks `mac`, shows the comparison code and the account's name, the player presses Approve | `mac` under `S`; a wrong one cancels the pairing |
| 6 | page → Central | `PairApprove { handle, sealed, proof }` → `Done` | claimed, not approved, same account and session, `proof` under `K` |
| 7 | client → Central | `PairPoll { handle }` → `Pending` or `Approved`; then `PairRedeem { handle, verifier }` → `PairSession { token, identity, username, sealed, expires_at }` | for the redeem: `SHA-256(verifier) = challenge`, approved, not redeemed; once |
| 8 | client | opens `sealed`, asks "Sign in as `<username>`?" | the seal's binding; a no ends the session (`Logout`) |

- **Handle**: 32 random bytes, base64url, stored only as a digest. A
  pairing lives five minutes from step 1; the claim (step 3) must come in
  its first two minutes, which is how long `S` is good for. `PairCancel {
  handle }` (page) ends it at once. Expired, cancelled or redeemed rows are
  deleted with their sealed key.
- **Claim** (client): an X25519 key pair (`sk`, `pk`), `v`, `challenge =
  SHA-256(v)`, and `mac = HMAC-SHA-256(S, "baylee-central/pair-claim/v1\0"
  ‖ issuer ‖ handle ‖ pk ‖ challenge)`. The client keeps the pairing's
  `expires_at` from `PairAccepted`. It polls `PairPoll` about once a second
  until it says approved, the pairing ended, or five minutes passed, and
  sends `v` once, in the redeem (RFC 7636's shape: the verifier crosses
  once, at the end).
- **Comparison code**: six Crockford base32 characters of
  `SHA-256("baylee-central/pair-code/v1\0" ‖ handle ‖ pk)`, shown by the
  client while it waits and by the page beside Approve. (An improvement over
  the owner's sketch: see "What `S` in the link means".)
- **Seal** (page): HPKE (RFC 9180) in base mode, DHKEM(X25519, HKDF-SHA256),
  HKDF-SHA256, ChaCha20-Poly1305, to `pk`, with `info =
  "baylee-central/pair-seal/v1\0" ‖ issuer ‖ account ‖ epoch ‖ handle ‖
  expires_at` (eight big-endian bytes each for epoch and time) and empty
  `aad`; the plaintext is the 32-byte master key. `sealed` is `enc` (32
  bytes) followed by the ciphertext (48 bytes): 80 bytes.
- **Proof** (page): `HMAC-SHA-256(K, "baylee-central/pair-approve/v1\0" ‖
  handle ‖ pk ‖ challenge ‖ SHA-256(sealed))`.
- **Session**: a native session like a login's, which ends the account's
  other native sessions as a login does. Its 12 hours count from the
  password proof behind the browser session (`proven_at`, below), never from
  the pairing. There is no state "session without key": the session exists
  only in the answer that also carries `sealed`, and only once.
- **Open** (client): HPKE open with `sk` and the same `info`: the issuer
  the client is built for, `account` and `epoch` from `PairSession.identity`,
  the handle from the link, and `expires_at` from `PairAccepted` (the
  pairing's, not `PairSession.expires_at`, which is the session's). Anything
  that does not open is refused and the session ended.

`proven_at`: `sessions` gains `proven_at timestamptz NOT NULL` (the time of
the password proof the session rests on; for a login, its creation), and
the 12-hour check moves from `created_at` to it (`expires_at <= proven_at +
interval '12 hours'`, `proven_at <= created_at`). A pairing's session takes
the browser session's `proven_at`; renewal never moves it.

The wire shapes are new variants of `central_api::wire::{Request,
Response}`; the browser operations join `browser_operation` and the
per-address budget table of `web-accounts.md` (`PairStart` 10 an hour,
`PairStatus` 120 a minute). On the native surface, which has only global
budgets, `PairClaim` and `PairRedeem` are bounded per handle (one claim, at
most 300 polls) and by a cap on open pairings (1000 in all).

### The link `baylee://pair`

`baylee://pair?h=<handle>&s=<S>`: both 43 base64url characters, exactly
these two parameters. `client-core::deeplink` reads it like the other links.
The client follows it only at rest and only while nobody is signed in at
Central; signed in, it says so and drops it (the pairing then expires).

### What `S` in the link means

`S` is a secret in a link, which the first part of this document avoided.
For its two minutes it is a bearer for whoever reads the link: on Windows
and Linux the link is the launcher's argument, which the player's other
processes can read, and an app that claims `baylee:` receives the link
instead of the client. Such a process could claim the pairing with its own
key and, approved, receive the master key. Three things stand in its way:
one claim per handle (if the real client also got the link, its claim fails
and it says so), the comparison code beside Approve, and the client's own
question. A process that reads other programs' arguments can usually read
their memory and keystrokes too, so this stays inside the existing local
trust boundary, but it is the reason for the code. The owner may drop the
code and the Approve press (the page then seals as soon as the MAC checks).

### The local chat vault (owner's item 1)

For a Central account the chat's device keys live in Central's encrypted
archive (`docs/central-chat-sync.md`). A password-wrapped local vault
(`password.vault`, `docs/password-vault.md`) is only an import source, and
today the chat starts only with the 120-second password hand-off
(`vault_access::Login::central`), so a paired sign-in would get no chat.

- The account worker derives the scope's local vault key from the master
  key and hands only that 32-byte key to the chat's restore job. The
  password hand-off and its 120 seconds go away for Central sign-ins.
- A vault version 2 (`BAYVAU02`) wraps the same data key under that key
  instead of under Argon2id of a password; the payload is unchanged.
- A version-1 vault is opened at the next password sign-in with that
  password, while it is still in the client for OPAQUE, and rewritten as
  version 2 (atomically, the data key and payload unchanged), then the
  password is dropped. Until then a paired sign-in imports nothing from it;
  the archive's keys work as before.
- Gateways with accounts of their own (no master key) keep version 1.

### Rotation (owner's item 4)

A new master key, every object sealed again under it, the epoch one higher.
The epoch is part of OPAQUE's context (`crypto-protocol.md`), so a new epoch
needs a new OPAQUE registration, which needs the password: rotation goes
with a password change ("offered at every password change"). "Automatic on
suspicion" means Central marks the account (`rotation_required`, set by an
administrator or by Central itself) and the next password sign-in rotates
before anything else. It never happens silently, and the OPAQUE server
setup is never regenerated.

1. `RotateStart { login, registration }`: KE1 under the current password
   and a registration request for the (possibly unchanged) password →
   `RotationChallenge { attempt, … }`; every object write is refused while
   an attempt is open (ten minutes).
2. The client makes a new master key, reads every object, seals each again
   under the new key and the new epoch, and uploads it to the attempt
   (`RotateObject { attempt, object, ciphertext }`, staged beside the live
   ones).
3. `RotateFinish { attempt, proof, record, wrapped_master, recovery,
   key_check }`: the proof under the current password, the new record and
   wrapper, the new `K`, and a new recovery wrapper (`Option`, with
   `EnrollmentFinish.recovery`'s rule: required for a registered account,
   refused for a legacy one; the old recovery code opens only the old key,
   so a new code is shown once). Central checks that
   the staged set is exactly the live set, then swaps credential, wrappers,
   `K`, epoch and objects in one transaction and ends every session; the
   answer is a new session.
4. The rotating client rewraps this device's version-2 vaults with the new
   key. Another device's version-2 vault opens with the old key only; its
   keys are in the archive, so it is rewritten from there at that device's
   next sign-in.

### Tests that must exist (attack cases)

Swapped key (sealed to another `pk`), a handle claimed twice, a reused
redeem, an expired handle (each step), a pairing approved with a wrong
`proof` or from another account's or session's cookie, a wrong epoch (the
account rotated between start and redeem), a `mac` under another `S`,
`SetKeyCheck` outside its two minutes or over an existing value, a session
that would outlive `proven_at + 12 h`, a rotation whose staged set misses or
adds an object, a reset to a new vault that leaves the old `K`, and RFC
9180's known-answer vectors for this suite in `mode_base` (Appendix A.2.1:
the KEM, the key schedule and the sealed messages, not the KEM alone).

### Dependencies (for the owner)

Central already links everything the seal needs: `curve25519-dalek` 4.1.3
(X25519 as `MontgomeryPoint::mul_clamped`), `hkdf`, `hmac`, `sha2` and
`chacha20poly1305` 0.11. Two ways:

| | What | New crates |
| --- | --- | --- |
| A (recommended) | HPKE base mode for the one suite, about 150 lines in `central-crypto`, pinned by RFC 9180 Appendix A's known-answer vectors for this suite, the all-zero DH output refused | none |
| B | `hpke` 0.15.1 (rozbb/rust-hpke, MIT/Apache-2.0, 28.10.2026) | `hpke` and what it pulls (its own X25519 and AEAD dependencies) |

A hybrid post-quantum KEM (ML-KEM with X25519) is not proposed now: HPKE
has no final hybrid suite in an RFC, and the sealed key lives five minutes.

### Who builds what

| Piece | Where | Who |
| --- | --- | --- |
| Derivations, claim MAC, comparison code, HPKE seal/open, proof | `central-crypto` | this session |
| Wasm exports of the page's half (verify `mac`, code, seal, proof, `K`) | `central-web-crypto` | this session, agreed with the website session |
| Requests, table and migration (`pairings`, `key_check`, `proven_at`, `rotation_required`), budgets, browser operations | `central-identity`, `central-api`, `central-migration` | this session |
| `central-client` SDK: claim, redeem, `SetKeyCheck`, rotation | `central-client` | this session |
| Holding the master key in an isolated worker, idle limit, the Approve page, the rotation UI | `web/` | website session |
| The `pair` link, the waiting screen, the question, `Account::from_pairing`, vault version 2, rotation in the client | Baylee | this session |

Order (PM, 30.10.2026): slice 1; then the vault key (item 1); then the key
check and pairing (items 3 and 2); then rotation (item 4). Landings only
after the PM agrees; production deploys are the PM's.

## Adding a gateway from a link

**Website.** Each card in the gateway directory gains "Add to Baylee" beside
"Copy address": `baylee://gateway/add?origin=` and the card's origin,
percent-encoded. The directory has to be served on the browser surface first;
today the page asks `/api/v1/native` and the boundary refuses it.

**Client, before asking.** The origin goes through `gateway::normalize`, plus
rules for links only: `https` only, a DNS name (no IP literal), no user
information, path, query or fragment. A `dev-control` build also takes a
numeric loopback `http` origin, for tests. The host is shown in its ASCII
(punycode) form, exactly as `normalize` prints it. The pinned gateway
(`gateway_list::PINNED`) or one already in the list: the client says it is
there and asks nothing. A full list (128) is refused with a sentence.

**The confirmation.** A dialog with the origin in full and these words, or
close to them: "A link asks to add this gateway. Gateways are run by other
people; add it only if you know who runs it. Your Baylee Central password
never goes to a gateway. If a gateway asks for a password, it is its own."
"Not now" is the default and is what `Esc` and `Enter` do; "Add" needs the
pointer or a deliberate key. Optionally the client asks Central's public
directory and adds "Listed in Baylee Central's directory" or "Not in Baylee
Central's directory". Listed does not mean trusted: Central keeps registered,
listed, verified and official apart (`docs/architecture.md` §"Gateways and
administration").

**After "Add".** The typed path, unchanged: `check_gateway`, `probe_gateway`,
`gateway_answered`. Signed out, the device's list is written; signed in at
Central, `sync_gateways` saves the account's list. No answer: "No gateway
answered at …" and nothing is saved. The row then shows the operator's name
and, under it, the origin (`row_words`), as for every gateway.

## Security review

| Threat | What stops it |
| --- | --- |
| A hostile page proposes a look-alike gateway. | The confirmation with the exact origin, "Not now" as default, the origin under any name in the row, never entered by itself. The pinned gateway cannot be replaced. A gateway that takes Central sign-ins never sees the password, and `Account::admit` refuses an unregistered gateway. |
| The look-alike asks for a password with its own form. | The confirmation says the Central password never goes to a gateway. This stays the real risk: people reuse passwords. |
| A page fires links in a loop. | The browser asks before it opens an app, per site. The client keeps one proposal, reads at most ten links a minute and drops the rest. |
| A link makes the client contact an address (an intranet host, a tracker). | Nothing is contacted before "Add"; links take `https` DNS names only. |
| Signing the client into someone else's account. | No link names an account. A pairing is started by the client and approved in the person's own browser session; the browser surface's `SameSite=Strict` cookie, `Origin` check and custom header guard the approval. |
| A stolen website cookie (good for up to 12 hours since sessions renew) approves the thief's own pairing. | `HandoffApprove` needs a recent password proof, not only the cookie. |
| Someone else starts a pairing and sends the person the page. | `K` goes to the person's own computer; the comparison code; with C1, the password. |
| Another app on the same computer claims the scheme. | It sees only public values, or `K` without `V`. |
| Another process reads a launcher's arguments. | Links carry no secret; `K` lives 60 seconds and is worthless alone. |
| Another process creates the channel first. | `FILE_FLAG_FIRST_PIPE_INSTANCE`; the socket in a `0700` directory. At worst it receives links, which hold no secret. |
| Malformed or huge links. | The strict parser, 2 KiB, a fuzz test over it. |
| A link ends up in a log. | The client logs a link's kind only; the website sends nothing anywhere else. |
| A pairing ends the other device's session. | As any sign-in does (one native session per account); the page says so before the approval. |

**What the website never puts into a link:** a password, a cookie, session or
token, the username, the account's id, an e-mail address, the recovery code,
a beta key, a gateway ticket, key material of any kind (wrapped or sealed), a
gateway's name or other text for the client to show, a redirect target. The
pairing code `K` appears only in the one return link of step 4, if question 3
allows it.

## Effort and order

| Slice | What | Where | Rough effort | Needs |
| --- | --- | --- | --- | --- |
| 0 | macOS spike: which process gets `application:openURLs:`, cold and warm; whether LaunchServices picks a generation bundle. | Baylee | 1–2 days | — |
| 1 | The scheme in all three installers and their checks; the launcher handoff; the client's channel and parser; `open`, `signin` (Central's form with a way back) and `gateway/add` with the confirmation, for every lobby state above. | Baylee | about a week after the spike | Questions 1 and 6. A manual reinstall for every player. |
| 2 | "Open Baylee" on the account page, "Add to Baylee" on the directory cards. | Central (`web/`) | a day | The directory served on the browser surface first. |
| 3 | Pairing: Central's requests, table, budgets, crypto, web page and SDK; `Account::from_pairing`; the lobby's pairing. | Both | two to three weeks with Central's assurance passes | Questions 2–4; the privacy notice. |
| later | Phones, with verified https links. | Both | — | Phone releases; question 7. |

Slice 1 is useful alone: from the website the client opens, and a gateway is
added with one confirmation. Slice 2 can follow at once, since it only writes
links the client already reads. Slice 3 is the only part that touches account
security.

## Questions for the owner

1. **Slice 1 needs a new launcher, so every player reinstalls by hand once.**
   Now, while players are few, or bundled with the next package that needs a
   manual install anyway?
2. **How does the client get the master key when signing in from the
   website?** (A) It does not: the link opens the client and the person types
   the password there. (C1) The website asks for the password again and pairs
   the client. (C2) The website keeps the master key while signed in (a change
   of `web-accounts.md`, and a window of up to 12 hours for any script on
   baylee.app, now that sessions renew) and pairs with one click until the
   tab reloads, and only shortly after the website's sign-in: Central has to
   see a recent password proof before it approves. Either way, a paired
   client's session ends 12 hours after the password proof behind it, never
   later.
3. **Does the pairing code in the return link conform to "no token in a
   URL"?** `baylee://signin/done?code=…` lives 60 seconds, is used once and is
   worthless without the client's verifier, which never leaves the client. If
   not, pairing rests on the comparison code only, which RFC 8628 §5.4 warns
   about.
4. **A new Central dependency for sealing the master key** (HPKE: the `hpke`
   crate, or `x25519-dalek` with the existing ChaCha20-Poly1305 and HKDF)?
5. **Which gateways may a link propose?** Any `https` origin with the
   warning, or only those listed in Central's directory? And should the
   confirmation say whether it is listed?
6. **The scheme's name.** `baylee:` is short and is what Central's roadmap
   says. For a sign-in return, RFC 8252 §7.1 requires a scheme made from a
   domain the app controls, written in reverse order: that would be
   `app.baylee:`. People never see it: the website
   shows buttons, and the browser asks "Open Baylee?". One scheme for all
   links, and which?
7. **Phones later, with verified https links** (App Links, Universal Links)
   rather than the custom scheme? Universal Links need the Apple Developer
   membership.

For legal: slices 1 and 2 keep nothing new about anyone. Slice 3 adds a
five-minute record on Central (handle, public key, sealed key, code digest)
and new per-address counters in memory; Central's privacy notice would name
them before it goes live.
