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
