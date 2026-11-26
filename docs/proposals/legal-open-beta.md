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
- **Minimum age 13**, the age Magic: The Gathering itself names, at
  Central too (owner, 25.11.2026: "Ja, Central soll auch auf 13 gehen").
  AceVik/baylee-central#92 (merged 25.11.2026) makes Central's terms and
  privacy policy version 2026-11-25 with 13, its registration and
  birth-date checks follow, and its privacy policy adds that a consent
  given under 16 needs the parents' agreement (Art. 8 GDPR; the gateway's
  5.3 already says so). 5.1 here names the same age for both and carries
  an `[OFFEN]` until Central's next release ships that version; until
  then Central asks for 16.
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
  14 days. **Done on the server on 25.11.2026** (owner's go-ahead):
  `/etc/logrotate.d/rsyslog`, `fail2ban`, `ufw` and `btmp` rotate `daily`
  with `rotate 13` and `maxage 14` (they were weekly with `rotate 4`, btmp
  monthly with `rotate 1`); the old configurations are in
  `/root/logrotate-backup-20261125/`, and the 12 archives older than 14
  days are deleted. Entries older than 14 days stay inside the last weekly
  archives until about 08.12.2026; the terms carry an `[OFFEN]` until
  then. The system journal already keeps 14 days (its oldest file began
  on 13.11.); the two journal files of a former machine ID (23.–26.09.)
  are deleted. The entries from 06.11. sat in the owner's own user journal
  (`user-1000.journal`: deploy output, sudo sessions, a build), which
  journald drops only as a whole; the owner chose to leave it (26.11.2026),
  since it holds no player data as far as checked. Before publishing,
  check once that the deploy output names no player. `wtmp` holds only
  the owner's own SSH sign-ins and is left as it is.

## Open points for the owner

Each one is marked where it sits in the draft.

8. **Backups:** the owner chose his private Google account (25.11.2026;
   Drive API with an OAuth client of his own), so there is no Art. 28
   agreement with Google. The backups are encrypted on the server before
   upload and Google never receives the key. 16.3, 9a.4, 16.6 (a new item
   for where backups are kept, naming Google Ireland Limited) and 16.8 now
   say so: undelivered messages at most 14 days in backups, everything
   else at most 12 months, deleted accounts deleted again after a restore, on Art.
   6(1)(f) with Art. 32(1)(c) GDPR. Two markers remain. The `[OFFEN]`
   says to publish only once the backup runs that way, and to name a
   second encrypted copy on the owner's own drive if one is added. The
   `[ANNAHME]` in 16.8 says the EU-US Data Privacy Framework carries
   storage outside the EU for a private account; that needs checking.
   Central's privacy policy § 6 needs the same recipient, and its
   sentence that transfers outside the EEA happen only with notifications
   turned on stops being true once the backup runs.
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
12. *(settled from the code)* **`ranked_run` retention:** the gateway
    deletes the link from a ranked game to its pairing once Central has
    accepted the result (`ranked::delivered`; a replicated match's a day
    after it was last seen, `ranked::replica::reap_absent`), and a task's
    link to its attempt likewise (`challenges::delivered`). 16.3 says so;
    the marker is gone.
13. *(settled, owner 25.11.2026: "now")* **Old friendships and groups on
    the gateway:** migration 35 (`m20261125_000035_drop_old_friends`)
    deletes every friendship, friend request, group, group membership and
    group invitation, and the undelivered messages of groups, on every
    gateway at its next start. No client has sent these to a gateway since
    the cutover; contacts and groups are Central's. Blocks stay, because
    the gateway still keeps a blocked player's table chat away with them
    and Central's blocks do not reach it; 9a.3 and 4.7 say so. **The
    release with migration 35 must be deployed before the new terms are
    published**, or 9a.3 says something that is not yet true. The routes
    under `/social/friends…` and `/social/groups…` and their store code are
    now dead on a Central gateway; removing them is a separate cleanup.
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
   paragraph under the title. Four of them are things to do on the
   server first: the journal capped at 14 days, the backup running as
   16.3 describes it, the two settings for reports to Central, and the
   release with migration 35 deployed.
2. Fill in `[NAME]`, `[E-MAIL]`, the version (a date) and the date, in a
   copy outside the repository.
3. Publish both languages in the console (`POST /admin/terms`,
   `docs/protocol.md` §"Terms of use (WG-1)"). Every player sees the new
   version at the next sign-in and accepts it again.
4. Replace the files here with the published text (still with `[NAME]`
   and `[E-MAIL]`) and note the version in `official/README.md`.
