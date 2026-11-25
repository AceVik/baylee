# Privacy notice and Impressum before the open beta (#316)

State of 25.11.2026, before the gateway opens beyond friends
(0.2.0-beta.o1). What exists, what this branch adds, and what only the owner
can settle. Not legal advice.

## The rule (owner, 25.11.2026)

A party's processing is described in the texts of the party the player has
the contract with. The official gateway's processing goes into the
gateway's terms of use, § 16 (`docs/gateway-legal/official/`); Baylee
Central's into Central's terms and privacy policy. A change to § 16 is
therefore a new version of the gateway's terms, which every player accepts
again at the next sign-in.

## What exists

| Where | What | State |
| --- | --- | --- |
| baylee.app | Central's Impressum, privacy policy, terms (`baylee-central/legal/`, the website) | live, version 2026-11-19 |
| gateway.baylee.app/site/impressum.html | the landing page's Impressum (§ 5 DDG), details from a private blob | live |
| gateway.baylee.app/site/datenschutz.html | the website's own privacy notice | live; linked `docs/privacy.md` for the game |
| the client, at first sign-in | the official gateway's terms with § 16 | live version 2026-10-21, describes beta.9b |
| `docs/privacy.md` | the developers' inventory, with file and symbol | current, but no notice to players |

The gap: the terms in force (2026-10-21) still describe local accounts with
passwords, beta keys, friends and groups on the gateway and reports to the
feedback service. `main` has moved all of that to Central.

## What this branch adds

- `docs/gateway-legal/official/terms.{de,en}.md`: first the live text of
  2026-10-21 as a baseline commit, then the draft for the open beta as a
  diff on it. The draft is marked `<!-- version: draft -->` and carries the
  open points inline as `[OFFEN: …]` / `[ANNAHME: …]`.
- `web/site/nutzungsbedingungen.html` with `terms.js`: the landing page shows
  the gateway's terms in force, read from `GET /terms`, so § 16 is
  reachable before anyone installs the client. The website's privacy notice,
  the Impressum and the start page link it (`#s16`) instead of
  `docs/privacy.md`, and every footer names it.
- `baylee-tools site checks`: the footer must link the terms page, and
  `terms.js` must never write markup.

## Open points for the owner

Each one is marked where it sits in the draft.

1. **Guests in the open beta?** Which `BAYLEE_AUTH_MODE` the official
   gateway runs (`central`, `mixed`). With `central` the guest passages
   (4.5, 9a.6, 16.3 "Gastkonto") go.
2. **Minimum age.** Central requires 16 for an account; the gateway's
   terms so far said 12 for everyone. Keep 12 for guests, or 16 everywhere?
3. **Administrators** besides the owner, who see reports and the console
   (§ 1, 16.6)?
4. **Hosted AI seats:** which providers the gateway offers at launch, named
   with their seat (8.11, 16.8).
5. **Processing agreement (AVV) with netcup** in place (16.6)? The website's
   notice has the same question open since 13.10.2026.
6. **Supervisory authority:** the LDI NRW is right only while the owner
   lives in North Rhine-Westphalia (16.9).
7. **Server logs, 14 days:** true only once journald no longer forwards to
   rsyslog or `/var/log/syslog` is rotated at 14 days; on 11.11.2026 syslog
   kept about five weeks (`baylee-central`
   `docs/proposals/website-text-review.md`). Either fix the server or
   write the real period.
8. **Backups:** is the plan still 48 h / 14 d / 8 w / 12 m?
9. **Reports:** the draft assumes the official gateway runs with
   `BAYLEE_REPORTS_TO=central` and `BAYLEE_DIRECT_REPORT_KEY`, so bug
   reports go to Central and reported chat messages to the feedback
   service (§ 10). Confirm, or the old § 10 stays.
10. **Ranked games are for people only** (7.5): `docs/protocol.md` says
    "two human seats"; confirm it is meant as a rule of play too.
11. **Replay sharing cut-off:** "on by default for games since 13.10.2026"
    (9.7); check the date against `replay_share_cutoff` before publishing.
12. **`ranked_run` retention:** the link from a ranked game to its pairing
    has no limit in the code (16.3). Keep it like the record, or limit it?
13. **Old friendships and groups on the gateway** (from before Central):
    they stay until the player deletes them; delete them on a fixed date?
14. **Deleting a Baylee account at Central** does not, as far as the code
    shows, delete the gateway's projection and decks. Either build that
    (Central tells the gateway) or keep the sentence that the player deletes
    the gateway's data in the client (4.7).
15. **`BAYLEE_IMPRESSUM_URL`** on the server: set it to
    `https://gateway.baylee.app/site/impressum.html`, so the client's
    About, terms sheet and network settings link the right page.

## Seen at Central, for its own texts

Central's texts are Central's (the rule above), so these are notes, not
changes made here:

- Its privacy policy names ranked placements on the public profile, but has
  no item for ranked duels themselves: the queue, the pairing, the rating
  and the result the gateway reports.
- The privacy text exists twice (`legal/privacy.*.md` and the website's
  Fluent messages); they can drift.
- Erasing a Tower expedition's echoes is not passed on where they were
  shared.
- Its Impressum says "Verantwortlich für den Inhalt: die oben genannte
  Person", the line of § 18 Abs. 2 MStV, which the landing page's Impressum
  left out on purpose because the site is no journalistic-editorial offer
  (13.10.2026, `docs/legal.md` §4). Harmless, but the two should agree.

## Publishing the new version

1. Settle the points above and remove every marker; remove the draft
   paragraph under the title.
2. Fill in `[NAME]`, `[E-MAIL]`, the version (a date) and the date, in a
   copy outside the repository.
3. Publish both languages in the console (`POST /admin/terms`,
   `docs/protocol.md` §"Terms of use (WG-1)"). Every player sees the new
   version at the next sign-in and accepts it again.
4. Replace the files here with the published text (still with `[NAME]`
   and `[E-MAIL]`) and note the version in `official/README.md`.
