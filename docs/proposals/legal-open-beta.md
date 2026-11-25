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

## Settled by the owner (25.11.2026)

- **No guests.** A gateway may allow guests in its admin console; it is off
  by default and stays off on the official gateway. The guest passages are
  gone; 3.5 and 4.5 say there are none, and that allowing them would be a
  new version.
- **Minimum age 13**, the age Magic: The Gathering itself names. Because
  every player needs a Baylee account, Central's minimum (16 today) applies
  on top; 5.1 says both.
- **Administrators:** only the owner for now; the definition keeps room for
  more and says "at present that is only me".
- **Hosted AI seats:** not named in the terms. The providers receive only
  the table's state without names (8.11); the room shows the provider at
  the seat.
- **AVV with netcup:** in place; 16.6 says so. **Supervisory authority:**
  the LDI NRW stands.
- **Server logs: 14 days, everywhere.** The owner asked how long German
  law lets them stay and to set every text and the server to that. No law
  names a number: the period is what the purpose needs (Art. 5(1)(e)
  GDPR), and the purpose here is running the service, finding faults and
  fending off attacks (Art. 6(1)(f)). What exists to measure against:
  - The BGH accepted seven days for an access provider's IP addresses,
    kept to find and fend off faults and attacks (III ZR 391/13,
    03.07.2014, after OLG Frankfurt 13 U 105/07, 28.08.2013).
  - The data protection authorities' working group on technology
    ("Orientierungshilfe Protokollierung") finds anything from a few days
    to several months acceptable, depending on the system, and wants the
    period fixed and justified.
  - Guides for site operators (eRecht24 and others) say "as short as
    possible" and usually name 7 to 14 days for logs with IP addresses.

  These logs name no person and hold an IP address only in rare error
  cases, so 14 days is well inside that range; five weeks would need a
  reason we do not have. 14 days is also what the journal already keeps
  (`MaxRetentionSec=14day`, checked read-only on 11.11.2026) and what
  Central's texts promise for the same server. So the gateway's terms
  (16.3 "Server logs" and "Moderation") and the website's notice now say
  14 days. **One server change remains before publishing:** the files
  rsyslog writes from the journal (`/var/log/syslog`, and fail2ban's log)
  rotate weekly with `rotate 4`, about five weeks. In
  `/etc/logrotate.d/rsyslog` and `/etc/logrotate.d/fail2ban`, replace
  `weekly` and `rotate 4` with `daily`, `rotate 13` and `maxage 14`
  (baylee-central `docs/operations.md` §"Logs and how long they stay").
  Then every text is true at once; the terms carry an `[OFFEN]` for it
  until then.

## Open points for the owner

Each one is marked where it sits in the draft.

8. **Backups:** there is no settled plan yet (owner, 25.11.2026); one is
   being worked out, with the copies on the owner's Google Drive. The
   draft no longer names tiers or 12 months; 16.3 says only that backups
   exist and carries an `[OFFEN]` for what the plan must add: where they
   lie and who processes them (Google under an Art. 28 agreement, a
   possible third-country transfer), encryption before upload, tiers and
   the longest retention, and the matching lines in 9a.4 and 16.6.
9. **Reports go to Central in the open beta** (owner, 25.11.2026). The code
   is done; the server needs `BAYLEE_REPORTS_TO=central` and
   `BAYLEE_DIRECT_REPORT_KEY` (at least 16 characters, the feedback
   service's `FEEDBACK_DIRECT_KEY`), beside the `BAYLEE_FEEDBACK_KEY` or
   `BAYLEE_FEEDBACK_TOKEN` it already has. Whether the live gateway runs
   so cannot be seen from the repository; the terms carry it as an
   `[OFFEN]` to do before publishing.
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
