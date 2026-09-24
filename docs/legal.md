# Legal & Licensing Policy

Private, non-commercial fan project hosted as open source on GitHub.
Not legal advice; for a public/commercial launch consult a Fachanwalt für
Urheber- und Medienrecht.

1. **Code license:** AGPL-3.0-only (`LICENSE`). `NOTICE` carries the fan
   content disclaimer.
2. **WotC Fan Content Policy:** the service is and stays completely free
   (no paywall, no paid features), and asks for no e-mail address, which
   the policy names beside payment as a price fan content may not charge:
   an account is a username and a password (#269; `docs/protocol.md`
   §"Signing in with a username"); clients show "unofficial fan content,
   not affiliated with Wizards of the Coast"; no WotC logos, no
   "Magic: The Gathering" in branding; no WotC asset is shipped or fetched
   except a card image under clause 3, and the symbols a client draws are
   clause 2a's subject rather than this one's.
   The same rule reaches everything the table is made of: the felt,
   the seat mats, the lobby's backdrop, the sky and **the weather over the
   table** are all computed rather than shipped
   (`crates/baylee-client/src/shaders/atmosphere.wgsl`). The weather is the
   sharpest case of it, because a falling leaf, a snowflake and a shaft of
   light are exactly the three things a renderer normally reaches for a
   downloaded sprite sheet to draw — so there is no `textures/` directory to
   audit, in the same way clause 5 leaves no `sounds/` one.
2a. **The symbols, and the one clause this project does not satisfy.**
   This used to read "mana symbols drawn by the client itself, or the
   open-licensed `mana` font — never WotC assets", and that sentence was
   wrong twice over. It was wrong about the code, because the client stopped
   drawing its own pips: `baylee_client_core::manapip` picks a glyph out of
   the Mana font and `crates/baylee-client/src/face.rs` only paints the disc
   under it. And it was wrong about the law, because an open licence on a
   typeface answers a different question than the one asked. Andrew Gioia
   licenses the Mana font under the SIL OFL 1.1 — confirmed in the upstream
   README, in the font's own `name` table (family `Mana`, v1.18,
   `license = SIL OFL 1.1`, designer Andrew Gioia), and by the bundled file
   being byte-identical to upstream's, so the Reserved Font Name clause is
   met. The same README says "All mana, tap, and card type symbol images are
   copyright Wizards of the Coast", and a licence cannot grant what its
   author does not hold. The OFL covers Gioia's *drawing* of the symbol; the
   symbol is Wizards'.

   Wizards are specific about which symbols. The Fan Content Policy's FAQ
   carries a table headed "a list of Wizards' most frequently asked about
   trademarks and logos that you may not include in your Fan Content", and
   the table is images rather than words, which is why reading the page
   rather than remembering it matters: beside the Magic, Arena and D&D
   logos it holds `MTG_PWSymbol.png` (the planeswalker symbol),
   `Mana_Symbols.png` and `guild_symbols.png`. The mana symbols are on the
   do-not-use list, by name, in the very policy this project claims to
   follow.

   So the honest statement is three tiers, not one rule:

   - **Named on that list — not used.** The planeswalker symbol, the guild
     and clan and family and school watermarks, any Magic or Arena logo.
     The Mana font draws all of them (499 mapped codepoints, far more than
     mana), which makes "it is in the font we already ship" the exact
     argument that must not be allowed to decide anything.
   - **Mana symbols and the tap symbol — used anyway, deliberately.** A
     client that cannot print `{T}: Add {G}` cannot show a player their own
     game. There *is* a lawful synonym — a coloured disc with a letter on it,
     which is nobody's mark, and which this client drew for a while — and the
     printed symbol is used instead, knowing that. The reason is that a mana
     cost is read at a glance and in a row, and the pictographs are what a
     player has been reading for thirty years; a rail of letters is a
     translation the reader has to perform. Every free tool in this space
     renders them (Scryfall, Moxfield, Archidekt, Forge, Cockatrice), and no
     such tool is known to have been asked to stop.
     This is therefore **tolerated, not permitted**, and the policy reserves
     the right "to stop or restrict your use of Wizards' IP at any time —
     for any reason or no reason". Under German law the descriptive-use
     carve-out (§ 23 MarkenG) is the argument a Fachanwalt would weigh; the
     preamble above already says that call is not made here.
   - **Everything else in the font — the same footing as the card text.**
     The Arena ability icons (flying, haste, hexproof, …), the card-type
     symbols, loyalty, saga, the counter marks: Wizards' graphics, and
     *not* on the trademark list. The policy permits Wizards' art and
     graphics in free fan content, which is the footing card names, oracle
     text and Scryfall's card images already stand on here. Using them in
     place of icons drawn for the purpose changes nothing categorically —
     it is one more thing on the pile clause 2 keeps free and unbranded.

   Two mechanical consequences for whoever writes the next glyph. A new
   codepoint is checked against that table **before** it is used, and the
   check is reading the policy page, not recalling it. And the glyph
   constants stay behind one door per purpose — `manapip::glyph` for the mana
   symbols, `cardrail::MARK_GLYPHS` for the keyword rail's twelve marks,
   `cardcrest::GLYPHS` for the three the identity column wears — because a
   scattered `'\u{e6xx}'` is a decision nobody can audit later. All three
   doors are in `baylee-client-core`, which draws nothing, so the set in use
   can be read without opening a renderer, and each door's doc comment
   carries the date its codepoints were held against the table.

   The third door was opened on 17.09.2026 and is the worked example of the
   rule. Three codepoints — `ms-commander` (E9C6), `ms-token` (E96D),
   `ms-ability-copy` (EA60) — read off the page rather than off memory: the
   table still holds fifteen images and none of the three is among them. The
   argument is *not* that the font's own stylesheet files `ms-commander`
   beside the card types; what that glyph draws is the Commander format
   symbol, and what clears it is the third tier — an expansion symbol on the
   same footing as the card images Scryfall already serves here. The
   planeswalker symbol at E623 is named on the table and is used nowhere.

3. **Scryfall:** honor rate limits (≤ 10 req/s), cache card images
   (encouraged by their terms), "data and images provided by Scryfall"
   attribution in clients. The cache is for our own players and never a
   mirror for anyone else, because their terms forbid republishing or
   proxying their data: the gateway's `/art` serves only a signed-in session
   and marks what it serves `private` (#273; `docs/protocol.md` §"Card
   art"). No card images are committed to the repo — and that includes the
   **card back**, which is fetched from Scryfall's own shelf for it
   (`backs.scryfall.io`) exactly like a printing's front rather than shipped
   as an asset. It is the same rule and it is worth spelling out, because the
   back is the one card image a client would be tempted to bundle: it never
   changes and every game needs it.
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
9. **The English Oracle, compiled in.**
   `crates/baylee-cards/src/generated_oracle.rs` holds the English Oracle
   text of every card in the pool, one string per face. It is not a new kind
   of thing in the tree: every card file already carries the same words as
   its `//! Oracle:` header, on the footing clause 2a names for card names
   and Oracle text — Wizards' material in free fan content — and the table
   is those words again, with the face boundary a header cannot mark. It is
   **not** clause 8's cache: it is written from the cache the way a card
   file is, and it holds the words and nothing else — no image, no price, no
   other Scryfall field. It exists because a client with no gateway and no
   network still has to show a player what an ability does, and the owner's
   rule is that what it shows is the card's own English, never a sentence
   the client made up.

The table UI icon map (`baylee-client-core/src/tableicons.rs`) was checked on
17.09.2026 against the same policy table and the upstream Mana 1.18 stylesheet
(https://github.com/andrewgioia/mana/blob/master/css/mana.css). It uses zone,
untap, sorcery, combat/ability and counter marks, with generic end/cleanup
controls from Font Awesome. It adds no logos or faction watermarks.
