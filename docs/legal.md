# Legal & Licensing Policy

Private, non-commercial fan project hosted as open source on GitHub.
Not legal advice; for a public/commercial launch consult a Fachanwalt für
Urheber- und Medienrecht.

1. **Code license:** AGPL-3.0-only (`LICENSE`). `NOTICE` carries the fan
   content disclaimer.
2. **WotC Fan Content Policy:** the service is and stays completely free
   (no paywall, no paid features); clients show "unofficial fan content,
   not affiliated with Wizards of the Coast"; no WotC logos, no
   "Magic: The Gathering" in branding; mana symbols drawn by the client
   itself (coloured pips, `crates/baylee-client/src/face.rs`) or, if a font
   is ever wanted, the open-licensed `mana` font (SIL OFL) — never WotC
   assets. The same rule reaches everything the table is made of: the felt,
   the seat mats, the lobby's backdrop, the sky and **the weather over the
   table** are all computed rather than shipped
   (`crates/baylee-client/src/shaders/atmosphere.wgsl`). The weather is the
   sharpest case of it, because a falling leaf, a snowflake and a shaft of
   light are exactly the three things a renderer normally reaches for a
   downloaded sprite sheet to draw — so there is no `textures/` directory to
   audit, in the same way clause 5 leaves no `sounds/` one.
3. **Scryfall:** honor rate limits (≤ 10 req/s), cache card images
   (encouraged by their terms), "data and images provided by Scryfall"
   attribution in clients. No card images are committed to the repo — and
   that includes the **card back**, which is fetched from Scryfall's own
   shelf for it (`backs.scryfall.io`) exactly like a printing's front rather
   than shipped as an asset. It is the same rule and it is worth spelling
   out, because the back is the one card image a client would be tempted to
   bundle: it never changes and every game needs it.
4. **Privacy:** self-hosted; minimal account data; account deletion
   endpoint; no tracking. As a private, GitHub-hosted open-source project
   no Impressum is required (no commercial/public telemedia service).
5. **Audio:** the client ships **no audio files**. Every sound it makes is
   computed — `crates/baylee-client/src/sound.rs` writes PCM and a RIFF
   header at startup, and nothing is fetched, bundled or sampled. This is
   clause 2's reasoning applied to the ear rather than the eye: ornament is
   the easiest thing to borrow by accident, and arithmetic borrows nothing.
   It is the same decision the table's felt, the seat mats and the lobby's
   backdrop were given, and it is the reason there is no "sounds/" directory
   to audit. A player's own sound pack is a later question and a different
   one — files a player supplies are theirs, not ours to distribute — and
   `baylee_client_core::cue::Cue` is deliberately a named moment rather than
   a file name so that answer stays open.
6. **AGPL §13 — the network clause.** This is the one licence obligation the
   project's own architecture triggers, and it was written down nowhere.
   §13 says a user who interacts with a modified version of the program
   *remotely, over a network* must be offered its Corresponding Source — and
   the gateway is exactly that: a process other people connect to.
   Publishing the repository is not by itself the offer, because a gateway
   may be running a patch nobody pushed. So the offer is answered by the
   running process: **`GET /source`** is unauthenticated (an offer
   conditional on having an account is not an offer to the people §13 is
   about) and names the licence, the version, the commit, the build number
   and the repository — plus `dirty`, the one field that says the commit
   does not fully describe what is running. `baylee-build` stamps all of it
   in at compile time, and the client prints the short form beside its own
   name in the lobby. Anyone who deploys a fork carries the same obligation
   and inherits the route that discharges it.
7. **The card-script reference.** Code generation may read an external,
   GPL-licensed corpus of rules scripts from a checkout the developer
   supplies. It is an automated lookup: no file of it is copied into this
   repository, vendored, or present in any build output, and nothing here is
   a derived work of it. `NOTICE` names the project precisely and is the
   only place that does — the code and the rest of the documentation call it
   "the corpus", which is a naming choice and not a claim about provenance.
   `xtask::scripts_root` finds the checkout instead of hard-coding a path,
   which is what keeps "never vendored" true of the build as well as of the
   tree.
8. **A cache of someone else's data is not source.**
   `data/scryfall-cache/` is one JSON payload per card in the pool, fetched
   under clause 3 and kept because Scryfall's guidelines encourage caching.
   It is **not committed** — 1371 files of it were, until it was taken out.
   CI restores it from its own cache and `xtask` refetches whatever is
   missing, so holding the line costs nothing.
