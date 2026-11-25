# The official gateway's terms of use

The terms of use of the official gateway, Baylee Sanctuary
(`https://gateway.baylee.app`, formerly `https://baylee.acevik.de`). Unlike
the default terms one folder up, which any operator may serve as they are,
these are the owner's own: longer, and with the gateway's privacy notice
(Art. 13 GDPR) as their § 16.

**Where the privacy notice lives (owner, 25.11.2026).** What a party
processes is described in the documents of the party the player has the
contract with: the gateway's processing in the gateway's terms (§ 16 here),
Baylee Central's in Central's texts (`baylee-central/legal/`, the website's
`/datenschutz`). So a change to § 16 is a new version of these terms, which
every player accepts again.

German is binding, English is a translation. The text uses only the Markdown
subset the client renders (`crates/baylee-client-core/src/terms.rs`, see
`../README.md`).

## Personal details

The operator's name and e-mail address appear in the published text, but in
no file of this repository: they stand here as `[NAME]` and `[E-MAIL]`, and
whoever publishes a version fills them in. Nothing else in these files is
personal.

## How a version is published

The official gateway takes its terms from the admin console (`POST
/admin/terms`, `docs/protocol.md` §"Terms of use (WG-1)"), not from
`BAYLEE_TERMS_PATH`. A version published there never changes; a new text is
a new version. `<!-- version: draft -->` marks a text that is not in force:
the gateway refuses it from a file, and it must not be published as it is.

## Versions

- **2026-10-21** (in force on the live gateway since 21.10.2026): the text
  the gateway served at `/terms` on 25.11.2026, with `[NAME]` and `[E-MAIL]`
  put in for the operator's details. It describes beta.9b: local accounts
  with passwords, guests, closed-beta keys, friends and chat on the gateway,
  reports to the feedback service.
- **draft** (25.11.2026, for the open beta 0.2.0-beta.o1; the files here
  now): the text brought to `main`. Baylee Central accounts (the gateway
  keeps no password), the closed beta over, friends, groups, personal
  messages and presence at Central, table, team and lobby chat on the
  gateway, ranked games and daily/weekly tasks, ban lists, sleeves and mats
  from the Central inventory, bug reports to Central, the new address, and
  in § 16 Central as a recipient and the right to object (Art. 21) as a
  section of its own (16.12). Before it is published, every `[OFFEN: …]` /
  `[OPEN: …]` and `[ANNAHME: …]` / `[ASSUMPTION: …]` is settled and
  removed, `[FASSUNG]`/`[VERSION]` and `[DATUM]`/`[DATE]` are filled in,
  the draft paragraph under the title goes, and the version comment becomes
  the date. The open points are listed in `docs/proposals/legal-open-beta.md`.

The landing page shows whatever version is in force, read from the gateway
(`web/site/nutzungsbedingungen.html`, `web/site/README.md` §"The terms of
use").
