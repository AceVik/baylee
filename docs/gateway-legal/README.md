# A gateway's terms of use and privacy notice

Texts for anyone who runs a Baylee gateway. German is binding, English is a
translation. Not legal advice: each operator is responsible for the terms of
their own gateway.

| file | what |
| --- | --- |
| `gateway-terms.de.md`, `gateway-terms.en.md` | default terms of use, usable as they are: they name the operator only through the gateway's legal notice |
| `gateway-privacy-template.de.md`, `gateway-privacy-template.en.md` | a privacy notice template: it states what the gateway software processes out of the box, and the operator fills in the parts in square brackets (who they are, hosting, how long they keep records, logs and backups) and adds what they do on top |

The default terms describe the software as it ships: chat with friends,
groups and at the table is end-to-end encrypted and the gateway cannot read
it, the lobby chat is public and not encrypted, every game is recorded in full and
the software deletes no record by itself, players choose per game whether
others see their hidden cards in replays.
An operator who adds a purpose (training a computer opponent, language-model
seats) says so, with its legal basis and how to object.

The text uses only the Markdown subset the client renders
(`crates/baylee-client-core/src/terms.rs`): `#`/`##` headings, paragraphs,
`**bold**`, `*italic*`, lists, rules and links; no tables, no HTML.

How a gateway serves and versions its terms, and how players accept them,
is described in `docs/protocol.md` §"Terms of use (WG-1)".
