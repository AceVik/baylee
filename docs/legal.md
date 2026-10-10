# Legal & Licensing Policy

Private, non-commercial fan project hosted as open source on GitHub.
Not legal advice; for a public/commercial launch consult a Fachanwalt für
Urheber- und Medienrecht.

1. **Code license:** AGPL-3.0-only (`LICENSE`). `NOTICE` carries the fan
   content disclaimer.
2. **WotC Fan Content Policy:** the service is and stays completely free
   (no paywall, no paid features), and asks for no e-mail address, which
   the policy names beside payment as a price fan content may not charge:
   an account is a username and a password, and a guest is a display name
   and nothing else (#269; `docs/protocol.md` §"Signing in with a
   username", §"Playing as a guest"). A guest may not upload a sleeve or a
   mat: an account anybody gets by asking cannot answer for a picture put on
   the gateway, so only a registered one may; clients show "unofficial fan content,
   not affiliated with Wizards of the Coast"; no WotC logos, no
   "Magic: The Gathering" in branding; no WotC asset is shipped or fetched
   except a card image under clause 3, and the symbols a client draws are
   clause 2a's subject rather than this one's.
   The same rule reaches everything the table is made of: the felt,
   the seat mats, the sky and **the weather over the table** are computed
   rather than shipped
   (`crates/baylee-client/src/shaders/atmosphere.wgsl`). The weather is the
   sharpest case of it, because a falling leaf, a snowflake and a shaft of
   light are exactly the three things a renderer normally reaches for a
   downloaded sprite sheet to draw — so there is no `textures/` directory to
   audit, in the same way clause 5 leaves no `sounds/` one.
   A card drawn from its text (#259) is the same case: its name bar, type
   bar, text box and P/T box are a card's functional layout, drawn by our
   own shader in flat colours with no frame art, ornament or symbol, so it
   borrows nothing from a printed frame and is not a grey area.
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
   symbols, `cardrail::MARK_GLYPHS` for the keyword strip's twelve marks,
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
   attribution in clients: `Phrase::ScryfallCredit`, under the Fan Content
   notice on the front door and under the version in the game menu, which a
   client seated straight into a game shows instead (#325). The cache is for our own players and never a
   mirror for anyone else, because their terms forbid republishing or
   proxying their data: the gateway's `/art` serves only a signed-in session
   and marks what it serves `private` (#273; `docs/protocol.md` §"Card
   art"), and `/catalog/text`, `/catalog/search`, `/pool` and `/printings`
   serve only a signed-in session too (#270; §"Card text"). The gateway
   asks Scryfall on nobody's behalf: it no longer fetches a printing its
   catalog lacks for whoever asked. A free account is enough, a guest's
   included, which the terms allow: "If you have an account system,
   end-users should be able to access card data anonymously or with free
   accounts." No card images are committed to the repo — and that includes the
   **card back**, which is fetched from Scryfall's own shelf for it
   (`backs.scryfall.io`) exactly like a printing's front rather than shipped
   as an asset. It is the same rule and it is worth spelling out, because the
   back is the one card image a client would be tempted to bundle: it never
   changes and every game needs it.

   **The feedback service's admin UI shows card pictures, and fetches none
   of them** (beta.7, 10.10.2026). A report names cards (`[Lightning
   Bolt]`, `docs/feedback.md` §"References in the text") and the set
   progress page lists a set's cards; the admin's browser loads each
   picture straight from Scryfall's own image host (`cards.scryfall.io`,
   `web/feedback/src/reports/scryfall.ts`), the URL built from the
   printing's id alone, under the same clause as the client's art: "You
   may not simply repackage, republish, or proxy Scryfall data". The
   service never fetches, stores, caches or passes on a card image, so it
   is neither a proxy nor a mirror; its CSP names that host as the one
   foreign image source and nothing else (`crates/baylee-feedback/src/web.rs`,
   `scryfall_is_the_one_foreign_origin_and_only_for_images`), `connect-src`
   stays `'self'` so the page cannot call Scryfall's API, and its referrer
   policy is `no-referrer`, so Scryfall sees a card id and nothing of a
   report. The pictures are seen by signed-in admins only, with "Image:
   Scryfall" under each, and are loaded on hover or on opening a set's list,
   never in bulk.

   **Nothing we paint lies on a card image** (#274). Scryfall asks that an
   image is not covered, cropped, tinted or stamped, and the artist's name
   and the © line run along its bottom edge. #274 drew each print whole in
   the window of a frame of ours and said everything about the card on the
   frame; the owner did not want the frame, and since #298 the print fills
   the card again and what the frame said lies on **objects** of ours, each
   with its own shadow, none of them paint in the image. Five of them touch
   a card, each by the owner's decision:
   - the **strip**, a small dark label lying on the art's bottom-left edge,
     where a modern frame's art meets its type line. The owner allowed it
     wider under #298, carrying the counter chip and the identity crests
     beside the keyword marks (the sleep moon it carried too went when
     summoning sickness became a wave, below),
     on one condition: it reads as a lifted object over the card with its
     own shadow. It never lies over the name, the cost, the type line or the
     artist (`cardrail::the_strip_lies_on_the_art_between_the_name_and_the_type_line`);
   - the **plate**, a card's power and toughness, loyalty or chapter where
     the print cannot say it, which left the strip on the owner's word of
     25.09 for the card's bottom right: a small dark plate with its own
     shadow, beside the printed box over the black border where its row
     leaves the room, on its own card in a fanned row (right-aligned to what
     of the card is in sight, its foot above the printed box, over the foot
     of its own text box), and upright under a tapped card in its lane's
     air (the PM, 25.09). It never lies over the name, the cost, the type
     line, the strip, the printed power/toughness box or the artist's and
     ©/™ lines (`cardplate::tests::the_plate_lies_on_nothing_its_card_says`,
     measured on 112 Scryfall scans), nor on another card's print: a tapped
     card whose neighbours' prints fill the air under it shows none
     (`layout::tests::plates::no_plate_lies_on_a_print_drawn_under_it`);
   - the **count badge** of a merged group, at the card's top-right corner,
     outside it, on the felt (the owner, 25.09: "the right edge and a bit
     higher"): over the card's top edge where the rows leave the room (a
     duel), beside its right edge where they do not (a ring), its shadow
     then ending on the print's own border. It never lies on another card's
     print either (the owner, 25.09). Over the card no card of its row
     reaches it, and it stays upright when its card taps; beside the card
     the gap after a merged card is held whole. A row too tight for that
     and a legible fan (at least a third of each card showing) scrolls
     sideways instead, in all three rows: it shows a run of whole cards,
     draws nothing of the rest, and has a scrollbar saying there is more
     (#298; `cardplate::nothing_of_the_badge_reaches_past_the_printed_border`,
     `table::badge_tests::no_badge_lies_on_another_cards_print`,
     `table::badge_tests::every_drawn_card_stands_inside_its_lane`). Until
     25.09 the tightest rows let it overhang the card before it, its
     top-right cost corner included;
   - the **offer's light** on the felt round a card, which the card lies on
     and which the next card of a fanned lane covers; the owner accepted
     that it shows only on the felt there.
   - the **shell** of a protected permanent: indestructible's steel rim,
     which stands round the card from just above its face down to the
     felt, hexproof's or shroud's dome of glass over it, and defender's
     brick wall on the felt past its top edge, lower than every card's
     face, and summoning sickness's wave of moonlight a hair over the
     face. Two rules hold them, by the owner's and the PM's decisions
     under #298. Over its own print the rim, a ring lying on the felt and
     the wall are exactly transparent: a mask follows the real camera's
     ray through each of their points to the card's face
     (`shellmat::the_mask_is_exactly_zero_over_the_print`, and
     `every_colour_but_the_domes_and_the_waves_carries_the_mask` holds
     every other return to it). **A dome is the one exception, by the owner's okay of
     25.09:** a real, tall dome of glass, nearly clear at its crown and
     deepening to its colour at its foot, may lie over its own card's whole
     print, the name and the artist and © line included. **So may the
     wave, by the same okay:** it passes over the print and leaves nothing
     behind, as the arrival sweep does, and where no crest is it draws
     nothing. The test holds that the dome's and the wave's branches each
     return exactly once and that nothing else goes unmasked. No shell
     ever lands on another card's print: where a shell standing round its
     card would reach another card's face this frame, a dome stands lower,
     then narrower, and lies down on the felt as a band of plates only
     last, the rim lies down as a ring, and the wave is not drawn
     (`table::shell_tests::a_standing_rim_never_lands_on_another_cards_print`,
     `a_standing_dome_never_lands_on_another_cards_print` and
     `a_wave_never_lands_on_another_cards_print`, swept over duels and
     rings, fanned rows, fliers, hovers and every seat's camera); the wall stands under every face, so every print in front
     of it hides it by depth (`a_wall_never_draws_over_a_print`, the same
     sweep), and so do the shadows a standing dome and the wall cast on
     the felt, which also carry the mask and are gone before a hover
     lifts them to a face (`a_shadow_lies_on_the_felt_outside_its_card`).
     The hover preview wears the same shells, standing, and holds the same
     rule: its rim and wall are exactly transparent over the preview's
     print, the mask being the card's own outline seen from straight over
     it, and only its dome and the wave lie over it
     (`shellui::tests::every_colour_but_the_domes_and_the_waves_carries_the_mask`).
   What reaches the image itself: its own **finish** (a foil is what that printing is, the one
   exception the owner accepted) and light that passes over the whole card
   and leaves nothing behind — the table's lamp pool, the one-second arrival
   sweep and the zone-change doors. The brushed coating every card used to
   wear lifted the image's blacks, the artist's line included, and was taken
   off. `cardmat::nothing_but_the_finish_is_drawn_on_the_print` holds both
   card shaders to that; `docs/client.md` §"The print fills the card" has
   the rest.
4. **Privacy:** self-hosted; minimal account data; account deletion
   endpoint (`DELETE /account`, `docs/protocol.md` §"Deleting an account
   (#292)"); no tracking. As a private, GitHub-hosted open-source project
   no Impressum is required (no commercial/public telemedia service).
5. **Audio:** interface/gameplay cues remain original computed PCM in
   `crates/baylee-client/src/sound.rs`. The revised five-suite score
   (10.10.2026) is newly composed for Baylee (AGPL-3.0-only), with five
   original two-bar themes. No melody or recording from the user's two Risen
   references is reproduced or distributed. Those private files were used
   only as reference context and for non-expressive signal statistics;
   no listening claim or legal-originality guarantee is made.

   Ten additional suites were added on 10.10.2026 under the owner’s explicit
   artistic freedom. Their authored motifs and native 48-kHz additive patches
   (including synthesized percussion) use no downloaded presets, soundfonts,
   loops, recordings or melodies. The original five remain unchanged. The
   additional acoustic layers use the same pinned, hash-verified CC0 bank.
   Selecting either bank affects recorded voices only; modelled instruments
   remain native 48 kHz. Purely synthesized arrangements sound identical in
   both bank settings. Policy and publisher pages below were re-read for this
   extension; no new third-party assets or licensing exceptions were added.

   The player uses the pinned **CC0 acoustic recordings** below for
   harp, plucked psaltery, violin, viola, cello, bowed bass and trombone.
   The lyre is our own plucked-string model. Every suite supports both banks:
   **Studio 48 kHz** (32-tap windowed-sinc preparation and pitch correction)
   and **Original 44.1 kHz** (original PCM with cubic playback). Both have
   **44.1-kHz recorded sources**; the 48-kHz bank is resampled, not native
   48-kHz recording. Output remains continuous at 48 kHz. This distinction
   is stated in Settings and the audition page. The user's revision explicitly
   requested the 44.1-kHz alternative; the implementation's main-bank
   assumption was explained in the chat pending their source preference.

   The [Wizards Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy),
   re-read 10.10.2026, says: “Don’t use Wizards’ Video or Music in your Fan Content.”
   No Wizards audio is used. This change adds no images, symbols, fonts or
   Scryfall API use. Both Scryfall terms URLs failed to load during this audit;
   existing image/API obligations continue, without claiming fresh verification.
   The publisher's [VSCO original-WAV page](https://versilian-studios.com/vsco-community/),
   checked 10.10.2026, states: “Licensed under CC0 (Creative Commons Zero) you can do whatever
   you want with these samples.” The [VCSL publisher page](https://versilian-studios.com/vcsl/)
   also identifies its collection as CC0. Source licences and recordist notices
   remain shipped beside the bank. Tests verify every active recording against
   its existing SHA-256, source revision and 44.1-kHz manifest entry.

   **Pinned recording sources:** the full historical bank remains intact;
   only the named acoustic families are used in the revised score. Unused
   instruments listed below are retained for provenance, not arranged into it.
   - **VCSL** (Versilian Community Sample Library, Versilian Studios LLC),
     [github.com/sgossner/VCSL](https://github.com/sgossner/VCSL) at
     `c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e`. Its README at that commit:
     “This collection is under a Creative Commons 0 license. Essentially
     it's Public Domain- you can do whatever you want with these sounds
     (even make commercial software), no royalties, no credit, no special
     terms.” Its `LICENSE` is the CC0 1.0 Universal legal code. Recorders,
     bowed and plucked psaltery, folk harp, strumstick, Renaissance organ,
     frame drums, bass drum (the davul), timpani (the nakers), rope-tension
     snare, tambourine, hand bells, finger cymbals, tubular bells, toms, a
     gong.
   - **VSCO 2 Community Edition** (Versilian Studios), not the paid
     editions, [github.com/sgossner/VSCO-2-CE](https://github.com/sgossner/VSCO-2-CE)
     at `440300901dfe9275fd84e0b7763af1f8443ae62e`. The
     [publisher's original-WAV page](https://versilian-studios.com/vsco-community/)
     states: “Licensed under CC0 (Creative Commons Zero) you can do whatever
     you want with these samples.” Checked 26.09.2026, together with the
     [source repository licence](https://github.com/sgossner/VSCO-2-CE/blob/440300901dfe9275fd84e0b7763af1f8443ae62e/LICENSE).
     Recordings: Sam Gossner and Simon Dalzell; sample editing: Elan
     Hickler / Soundemote. Solo violin, solo contrabass (arco and
     pizzicato), the violin, viola and cello sections (sustained and
     spiccato), French horn, tenor trombone, tuba, a timpani roll, and from
     the bundled "VSCO 1 Percussion" folder (same repository, same licence) a
     large ethnic drum, a bass drum and its roll, and a suspended cymbal's
     roll (music v2's orchestral body, 09.10.2026).
   - **FreePats Bagpipe**, [github.com/freepats/bagpipe](https://github.com/freepats/bagpipe)
     ([freepats.zenvoid.org](http://freepats.zenvoid.org/Ethnic/bagpipe.html))
     at `496f2f6e82f226d650e270c0f0ad5febbebec249` (release 2026-08-06). Its
     README at that commit: “Published under the terms of Creative Commons
     CC0 public domain dedication:
     https://creativecommons.org/publicdomain/zero/1.0/”, and “Samples
     recorded on August 2020 by Gilles Sadowski, on a bagpipe in G designed
     by Rémy Dubois and built by Olle Geris.” Its `LICENSE.txt` is the CC0
     1.0 legal code. Chanter and both drones.

   `art/music/samples.json` records for every recording its source, path,
   pinned revision, SHA-256 of the original, the prepared PCM's SHA-256 and
   its pitch and loop facts; `art/music/prepare.py` rebuilds the bank
   reproducibly (mono 44,100 Hz PCM16, sustains looped). Shipped beside the
   bank in `crates/baylee-client-core/assets/orchestra/`: `LICENSE-CC0.txt`
   (VSCO 2 CE's), `LICENSE-CC0-VCSL.txt`, `LICENSE-CC0-FreePats.txt` (each
   source's own file, byte for byte) and `NOTICE` naming the three sources
   and their recordists, which CC0 does not require and we keep anyway. A
   former bank test checked every row against these pinned sources; the active
   model renderer instead tests tuning, spectral tables and bounded playback. The piano, oboe, orchestral
   strings, timpani, snare and suspended cymbal of the earlier VSCO 2 CE bank
   left with music v2 (owner, 08.10.2026).
   Considered and **not used**: the Polyphone soundfont “Early European
   Instruments” (stated “public domain” by an uploader whose samples came
   from “various records”: no provenance to stand on); single Freesound
   uploads marked CC0 (one uploader's word and no second source); and every
   non-CC0 library (MF Tin Whistle CC BY-NC-SA, Karoryfer's commercial sets,
   Garritan, Philharmonia, Iowa MIS, Pianobook, Sonatina).
   Every melody is written for Baylee; the styles named in the brief
   (British and Slavic folk, medieval European music, fantasy scores, a
   cinematic hybrid orchestral sound) are references in words only: a
   style may be emulated, a work may not. No theme, ostinato or chord
   progression of any film or game score is quoted or paraphrased; the
   best-known ones are on the avoid list as interval shapes, checked against
   the independently notated melodic phrases (a limited resemblance check,
   not proof that no existing music is similar). `art/music/avoid.json` holds the openings of
   tunes we must not echo, as directed intervals, and a test fails any score
   melody that shares six of them in a row. No game soundtrack, melody or
   recording from Wizards or Blizzard is used.
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
   and inherits the route that discharges it; a fork that publishes its
   source somewhere else says where with `BAYLEE_SOURCE_URL`. `GET /info`,
   which every client asks before it signs in, carries the same address as
   `source` (#270), so a client can show it where players are rather than
   only where somebody knows to look. The client's front door draws it
   under the Fan Content notice, on every panel: the address the gateway
   it points at gave, else the client's own repository
   (`lobby::front::source_address`) — as text beside the full notice on a
   desktop window (Wide, Vast), and on the smaller classes as the one-line
   colophon's **Source (AGPL-3.0)**, a press away from the address itself
   (see §"The front door's notices", below). Since #299 it is a link that opens
   the address in the player's browser on their click, with the same
   address as a QR code under it on a screen a phone can be held up to.
   Both are drawn only for an address that passes the client's own check
   at the door (`gateway_info::web_address`: plain `http(s)`, at most 200
   characters, no whitespace, control or bidi character), because the
   gateway that sent it may be hostile; the whole address stays in sight
   as text, so what a phone reads is what the screen says.
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
10. **Baylee's identity and sanctuary (26.09.2026).** The owner requested
    a logo, app icon and richly detailed animated login inspired by the sense
    of crossing a threshold in *World of Warcraft: Midnight*. They explicitly
    permitted generated textures and Baylee as a mascot. The result is an
    original moonlit conservatory garden with limestone architecture, water,
    botanical reliefs and warm lamps; it replaces the earlier procedural geode.
    No Blizzard or Wizards image, texture, model, logo, font, character or
    recognizable location is copied, traced, sampled or bundled.

    `art/baylee/sanctuary-master.png`, `assets/scenes/sanctuary-world.png`,
    `sanctuary-frame.png`, `baylee-guardian.png` and the two brand
    masters are generated with the built-in image tool. The cat's identity
    references the owner's supplied photographs; those photos and the people
    appearing in them are not shipped. `wayfinder-lantern.png` is a render of
    our original Blender model (`art/baylee/wayfinder-lantern.blend`); its
    source script and GLB export accompany it. Real-time water, mist, light
    and particles are original WGSL. Exact prompts and asset provenance are
    recorded in `art/baylee/`. Project-authored materials are distributed under
    the repository's AGPL-3.0-only licence, to the extent rights apply. No
    third-party stock artwork or additional font is introduced.

    Checked the [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
    on 26.09.2026: “Please respect other people's IP.” Its FAQ prohibits
    incorporating Wizards' logos and trademarks without written consent;
    this identity contains only Baylee and original lettering/ornament.
    The existing unofficial-content notice remains on the front door. Its later
    layout revision changes whitespace only, retaining all words and punctuation
    in three centred sentence lines. Clause 2 was checked again directly:
    “Tell the Community it’s unofficial.” A regression test compares the quoted
    notice after whitespace normalization. Water/cloud shader geometry and the
    shared lobby/editor view use only the existing original project artwork. The
    later empty-state card backs are original UI geometry (rounded rectangles
    and diamond inlays), with no card images or third-party emblems. The same
    policy's “Please respect other people's IP.” was checked directly for this
    addition; the notice, bundled font licences and source link are retained.
    Checked the [Scryfall API and image rules](https://scryfall.com/docs/api)
    directly (HTTP fetch after the browser returned 403): “Do not add your own
    watermarks, stamps, or logos to card images.” This logo and scene never
    sample or alter a card image. The existing bundled Alegreya Sans and
    Faustina notices state: “This Font Software is licensed under the SIL
    Open Font License, Version 1.1.” Those fonts remain unmodified; the
    generated logo does not bundle a new font. No policy exception is needed.
11. **Preconstructed deck lists (MTGJSON, 30.09.2026).**
    `data/decks/precon/` holds the card lists of Wizards' retail decks
    (theme, intro, commander, duel decks and the rest `docs/precons.md`
    names), written by `xtask decks-import` from MTGJSON's deck archive.
    MTGJSON's licence page, read on 30.09.2026
    (https://mtgjson.com/license/): “By using this website and its content
    you agree to the following License: Copyright © 2018 – Present, Zach
    Halpern Permission is hereby granted, free of charge, to any person
    obtaining a copy of this software and associated documentation files
    […] to deal in the Software without restriction, including without
    limitation the rights to use, copy, modify, merge, publish, distribute
    […] subject to the following conditions: The above copyright notice and
    this permission notice shall be included in all copies or substantial
    portions of the Software.” So the notice and the permission text are in
    `NOTICE`, and every file's header names the source, the licence and
    `NOTICE`.
    What is kept is the least the lists need: a count, the ledger's card
    name, the printing (set and collector number, language, foil) and, per
    file, the product's name, type, set code and release date. No price,
    image, rules text, flavour text or any other MTGJSON field is kept, and
    the 260 MB archive is a gitignored cache (`data/mtgjson-cache/`, the
    same footing as clause 8). The card names are Wizards' material on
    clause 2a's footing, exactly as a card file's header is.
    `https://mtgjson.com/robots.txt` says `Disallow: /api/v5/*.json` to
    every agent, so neither `DeckList.json` nor any per-deck `.json` is
    fetched: the importer downloads the one archive
    (`AllDeckFiles.tar.gz`), which that line does not cover. Some product
    names are people's names — the World Championship and 1996 Pro Tour
    decks are sold under their players' names — and are kept as the product
    titles Wizards printed; no user or author of any deck site is stored.
    Community sources are researched, not imported (`docs/precons.md`
    §"Community decks").


The table UI icon map (`baylee-client-core/src/tableicons.rs`) was checked on
17.09.2026 against the same policy table and the upstream Mana 1.18 stylesheet
(https://github.com/andrewgioia/mana/blob/master/css/mana.css). It uses zone,
untap, sorcery, combat/ability and counter marks, with generic end/cleanup
controls from Font Awesome. It adds no logos or faction watermarks.

### Room and gateway interface review (2026-09-26)

The room controls and textured gateway surfaces are original code; no new
third-party artwork, logos, fonts or audio are added. The existing fan-content
and source notices remain visible in the login footer. The owner requested
that these notices appear only in the login area, preserving room and lobby
space for play. Checked the
[Wizards policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy):
“Tell the Community it’s unofficial.” The FAQ also explicitly says:
“Yes. You can require a login/sign up to play a private group game.”
Optional room passwords use the existing private-group access model; guest
access and the public source offer remain available. Starting permanents use
the existing registry and licensed image-delivery path documented above.

The room's starting-card dropdown and badges reuse the existing licensed card
image loader and catalog printing picker; they add no bundled card artwork.
The 2026-09-26 review reconfirmed the policy wording: “Don’t mess with the legal
notices in our stuff.” Whole card thumbnails/previews preserve those notices,
and finish choices use the already recorded foil treatment approval above.
Scryfall's API documentation returned HTTP 403 during this review; the existing
catalog/cache/rate-limit path and its previously documented terms are unchanged.

The native app title and Dock icon reuse the original Baylee branding already
shipped in the bundle. No third-party logo is added. Rechecked the
[Wizards policy](https://company.wizards.com/en/legal/fancontentpolicy) on
2026-09-26: “Don’t use Wizards’ logos and trademarks.” The Baylee icon depicts
the owner's cat, moon and flowers; the existing fan-content notice remains.

## Curated evaluation decks (2026-10-01)

The fifteen lists in `data/decks/eval/` are original deck selections over
Baylee's existing implemented, tested pool. Printing references come from
the committed card headers; these files contain counts and printing
identifiers, with no images, symbols, fonts, audio or copied corpus scripts.
The repository licence and `NOTICE` continue to apply.

Checked the [Wizards policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
and [Scryfall API and image terms](https://scryfall.com/docs/api) directly:
“Tell the Community it’s unofficial.” The existing notice remains in place.
Scryfall requires: “Your software must create additional value for end-users.”
These curated archetypes supply repeatable evaluation matchups, rather than
republishing a card database. No image is added or altered and no new
third-party asset licence or policy exception is introduced.

## Feedback interface corrections (2026-10-01)

The copied-token lookup reuses existing registered token printings, matching
name, base power/toughness, colour and card types. It adds no bundled art.
Charge-counter labels sit outside the card image; stack scrollbars, attack
controls and the finite yes-series button are original interface elements.
The existing font files and their bundled SIL Open Font License notices are
unchanged. The OFL permits embedding and redistribution provided each copy
contains the copyright notice and licence (condition 2).

Checked the [Wizards policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy):
“Don’t mess with the legal notices in our stuff.” The existing unofficial
notice remains. Checked [Scryfall's image terms](https://scryfall.com/docs/api):
“Do not cover, crop, or clip off the copyright or artist name on card images.”
The full-image rendering path remains, and the counter label is outside the
print. No new asset or policy exception is introduced.

The follow-up public hand-limit badge uses only original localized interface
text and the existing font's infinity character. It sits in the player's
status area, outside card images; the same quoted rules and font licence apply.

The remaining-combat-damage button likewise uses original localized interface
text and existing fonts. It is drawn in the prompt bar outside all card images;
the reviewed policy, full-image path and bundled font licence remain applicable.

Life-total fitting changes only original status text, using the existing font
files and preserving complete card images and all notices.

Target filters are original localized controls. Choice-button previews reuse
the existing full-card image renderer and licensed image delivery, keeping
artist/copyright lines intact; the battlefield cue uses the existing hover
transform. No new external asset, font or policy exception is introduced.


Food stack presentation (2026-10-02) reuses the full registered ELD token
image and its verified Oracle sentence from
[Scryfall's token record](https://scryfall.com/card/teld/15/food).
The sentence is displayed beside the image, with no bundled artwork.
Rechecked the policy/FAQ and Scryfall page quoted above: “Don’t mess with the
legal notices in our stuff.” and “Do not cover, crop, or clip off the copyright
or artist name on card images.” Existing fonts, attribution and notices apply;
no new asset licence or exception is introduced.


Readability review (2026-10-02): larger previews, text and less-overlapping
rows change layout only, using the existing fonts and symbols under their
recorded licences. The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were rechecked: “Don’t mess with the legal notices in our stuff.”
[Scryfall API/image terms](https://scryfall.com/docs/api), retrieved directly,
require “Do not cover, crop, or clip off the copyright or artist name on card images.”
The preview still fits the entire card at its original aspect ratio; native
QA includes its bottom artist/copyright line. No new asset, symbol or legal
exception is introduced.

The follow-up camera/HUD pass uses the same bundled OFL fonts and original
geometry. It reduces decorative table margin, keeping complete card images
and notices. No additional assets or licence exceptions are needed.

Score follow-up (2026-10-02): only original note scheduling and orchestration
changed. The existing VSCO 2 Community Edition bank is unchanged. Rechecked
the [publisher's sample licence](https://versilian-studios.com/vsco-community/):
“Licensed under CC0 (Creative Commons Zero) you can do whatever you want with
these samples.” Attribution and the bundled CC0 dedication remain intact;
no third-party melody or additional recording was imported. The login fade
changes original procedural button surfaces only.

The decision clock beside each player's plate (2026-10-08) uses the `clock`
glyph (U+F017) already in the bundled Font Awesome Free font, under the
same licence line quoted below; it is drawn on our own pill on the table's
overlay, never on a card image.

Compact charge labels (2026-10-02) use the battery-half glyph already in the
bundled Font Awesome Free font, distinct from the player's energy symbol.
The [upstream licence](https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/LICENSE.txt)
says: “the SIL OFL license applies to all icons packaged as web and desktop
font files.” Existing font notices and embedding remain unchanged. The
label sits outside the card image; full localized counter text appears on
card hover. No new font or artwork is shipped.

The compact waiting-room pass (2026-10-02) rearranges original controls and
text using the same bundled fonts. Card thumbnails and hover previews use
the unchanged full-image path. The policy and image-term quotations above
continue to apply; there is no additional asset or licence exception.

Annotation collision placement additionally keeps text outside the complete
hover-preview rectangle. The full image and its artist/copyright lines remain
visible in the 1280×800 native comparison. This is layout arithmetic only;
no new asset, font or symbol is introduced.


## Rules-implementation policy gap found during Alpha review (2026-10-02)

The owner explicitly authorized Codex to implement the remaining Alpha cards
and their tests, overriding the model assignment in CLAUDE.md:179 for this task.
That authorization does not establish permission from Wizards.

On re-reading the [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy),
the answer to “Can I use all of Wizards’ IP?” expressly includes “game mechanics”
among uses requiring “our prior written permission”. This is a broader restriction
than the image/font/notice checks documented above. No such written permission
was found in AGENTS.md, CLAUDE.md, NOTICE or this document. This records a conflict
with the project's claimed policy basis, not a conclusion about copyright law.

A concrete existing example is the triggered land-tap/damage implementation in
`crates/baylee-cards/src/cards/enchantments/mv_4/manabarbs.rs:25`; completing
Power Surge would add another rules implementation. AGENTS.md's legal guardrail
requires stopping and telling the owner when a requested change conflicts with
an explicit prohibition. Further card implementation is therefore held pending
clarification of the applicable permission or a reviewed change to that project
rule. Existing code is preserved; nothing was deployed or deleted. No broader
legal audit or claim that earlier policy checks settled this issue is implied.


### Owner decision: rules implementation as tolerated practice (2026-10-02)

The owner explicitly confirmed that this project proceeds as a tolerated grey
area, citing comparable community rules simulators, and authorized continuation.
This is acceptance of that uncertainty, not a claim of written Wizards permission
or proof that the Fan Content Policy grants the use. The quoted “game mechanics”
and “our prior written permission” restriction above remains documented beside
this decision. For this task, the owner-approved treatment is analogous to the
existing tolerated-use exceptions: continue the non-commercial rules engine and
card implementations while preserving notices, attribution and asset restrictions.
The temporary hold recorded above is lifted by this explicit owner instruction.


The Black Vise completion (2026-10-02) follows that owner decision. Its new
client annotation is ordinary seat-name text in the existing licensed fonts;
full card images, notices and attribution use the unchanged rendering path.
No artwork, symbol or font asset is added.


Mana Flare (2026-10-02) follows the same owner-approved rules-implementation
exception above. It adds no assets and uses the existing mana-choice UI,
licensed fonts/symbols, complete card rendering and attribution unchanged.

Balance (2026-10-02) follows the owner-approved rules-implementation exception
above. Its keep prompt and public-choice log are plain text in the existing
licensed fonts. No artwork, font, symbol or other asset is added; the complete
card-image and attribution path is unchanged.

Gloom (2026-10-02) follows the owner-approved rules-implementation exception
above. Its cost increases and planning hints add no assets or player-facing
symbols. Card images, attribution, fonts and existing mana symbols are unchanged.

Cyclopean Tomb (2026-10-02) follows the same owner-approved rules-implementation
exception above. Its new mark-removal prompt is ordinary text using the existing
licensed fonts. No card artwork, symbols, font or other asset is added.

Creature Bond (2026-10-02) follows the owner-approved rules-implementation
exception above. The policy's instruction “Tell the Community it’s unofficial.”
([official page](https://company.wizards.com/en/legal/fancontentpolicy), checked
2026-10-02) remains satisfied by the existing project notices. This rules/test
change introduces no artwork, symbol, font or other shipped asset; the existing
Scryfall attribution and complete-image rendering path is unchanged.

Consecrate Land (2026-10-02) uses the same recorded owner exception and the
policy/asset checks above. Only card rules, engine checks and tests changed;
no new artwork, font, symbol or other asset is shipped.

Animate Artifact (2026-10-02) uses the same recorded owner exception. The
official policy and FAQ were reread; “Tell the Community it’s unofficial.”
remains covered by the existing notices. This change adds only rules and tests;
no artwork, font, symbol, image distribution or attribution behavior changes.

Nether Shadow (2026-10-02) uses the same recorded owner exception. The
[policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy) were
reread: “Tell the Community it’s unofficial.” remains covered by the existing
notices. The graveyard-order instruction is plain text in the existing licensed
fonts; no new asset or change to the image/attribution path is introduced.
The bundled Alegreya Sans and Faustina licenses state: “This Font Software is
licensed under the SIL Open Font License, Version 1.1.”

Sunglasses of Urza (2026-10-02) follows the recorded owner exception and policy
review above. Its rules and mana-planning changes introduce no shipped assets,
symbols or fonts and do not alter complete-image rendering or attribution.
The existing unofficial notice still satisfies “Tell the Community it’s unofficial.”
from the [official policy](https://company.wizards.com/en/legal/fancontentpolicy).

Sengir Vampire (2026-10-02) follows the same recorded owner exception. The
[official Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
and [Scryfall API/image terms](https://scryfall.com/docs/api) were re-read.
“Tell the Community it’s unofficial.” remains satisfied by the project notices;
“Do not cover, crop, or clip off the copyright or artist name on card images.”
remains unchanged by this rules-only completion. No assets, fonts or corpus
source files are added.

The Shift-preview correction (2026-10-02) reuses the existing image cache and
selected printing: only the requested face changes, while print id, language
and finish stay identical. No card images are bundled or newly licensed assets
introduced. The [Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy),
re-read today, says: “Don’t mess with the legal notices in our stuff.” The full
reverse image retains its notices just like the front. Foil continues under the
already recorded owner approval. The Scryfall API terms endpoint again returned
HTTP 403 during this check; the existing image-delivery restrictions above
remain in force, without claiming a fresh successful read of that page.
The preview's keyboard hints use the already licensed UI font and original,
simple line geometry for Shift and Option; no additional icon or font asset is
shipped, and the hints sit outside the complete card image.

Earthbind (2026-10-02) follows the recorded owner exception and today's official
policy review above. Its rules and tests add no assets, font, artwork or corpus
source files. The policy requirement “Don’t mess with the legal notices in our
stuff.” remains satisfied by the unchanged complete-image rendering path.

The remaining-25 Alpha completion pass (2026-10-02) follows the same recorded
owner exception. The [official policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were reread for this pass: “Tell the Community it’s unofficial.” is covered by
the existing notices. Oracle and rulings are fetched for local verification;
no image, font, audio or corpus source asset is added. The Scryfall API terms
page still returns HTTP 403; no fresh successful terms-page read is claimed.

The same pass extends the existing Mana-font rendering path to gameplay
prompts and error text in `hud/ledge.rs::sentence`, where brace tokens could
previously appear as literal text. No new font, glyph artwork or external
asset is added. The recorded owner exception for the Mana font in §2a applies.

## Desktop installers (2026-10-06)

The installers (`docs/releasing.md` §"Installers") add three pictures, drawn
by `scripts/installers/make-art.py` and committed: the dmg window's
background (a gradient, eight dots, an arrow and two lines of text set in the
bundled Alegreya Sans), and the Windows `.ico` and Linux icon, both resized
from the existing Baylee brand icon. No third-party art, logo or symbol is
used. The [Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy)
was reread on 2026-10-06: “Don't use Wizards' logos and trademarks.” None is
used. “Tell the Community it's unofficial. Make it clear that your Fan
Content is not endorsed or sponsored by Wizards—i.e., unofficial.” The dmg
background says “Unofficial fan content, not affiliated with Wizards of the
Coast.”, and so does the `.deb`'s description; the installed tree is the
archive's, with its `README.txt`, `LICENSE` and `NOTICE`. The fonts' licence
says: “This Font Software is licensed under the SIL Open Font License,
Version 1.1.”; text set in them inside a picture is a document made with the
font, not the font. dmgbuild, appimagetool and dpkg only build the files.

## A rounder app icon (2026-10-08)

The owner asked for a rounder, less square icon on 08.10.2026, and reported
it missing on Windows. Every app icon
(`baylee.icns`, the Windows `.ico`, the Linux hicolor PNGs and the window
icon) is now the same Baylee brand painting, the owner's cat, cut by
`scripts/installers/make-art.py` to a rounded square whose corners are a
plain superellipse, with transparent corners and, on macOS, a soft shadow.
The mask and the shadow are geometry computed by the script; Apple's 824 of
1024 grid is a proportion, not an Apple asset. No third-party art, logo,
template or symbol is used, so the Fan Content Policy line quoted in the
2026-10-06 entry above (“Don't use Wizards' logos and trademarks.”) holds
unchanged. The executables now carry the same icon as a Windows resource, and
their version information names the program "Baylee"; nothing in it claims
an affiliation.
Two tools put a program part of their own into what we publish: Inno
Setup's setup stub, under the Inno Setup licence (“Permission is granted to
anyone to use this software for any purpose, including commercial
applications, and to alter and redistribute it”, provided the copyright
notices it carries stay in place, which an unmodified stub does), and the
AppImage type2 runtime (MIT; its README: “a statically linked runtime …
Since the runtime is linked statically, libfuse2 is no longer required”).
Open for the owner: the runtime statically links FUSE code, whose upstream
libfuse is LGPL-2.1; what that asks of a project that redistributes the
pinned runtime binary (20251108, sources public at that tag) has not been
reviewed here.

## The front door's notices (WP1, 2026-10-07)

The shell redesign (`.claude/ux-b6/DESIGN-v5.md` §3, principle 8, owner-
approved 06.10.2026; its Q12 recommends exactly this amendment) keeps the
notices on every face of the front door at every size class, and changes
where their *full* text stands on the smaller ones:

- **Wide and Vast** (a desktop window): under the card, on a mist plate,
  the Fan Content notice **word for word**
  (`lobby::front::FAN_CONTENT_NOTICE`, pinned by
  `the_notice_is_the_policy_s_own_words`) and Scryfall's credit, set as a
  short centred paragraph; and in the window's **top-left corner**, on a
  mist plate, the source offer: its QR code, "Source code (AGPL-3.0)" and
  under it the address, the whole tile a link to it. Moved to a corner on
  09.10.2026 at the owner's request (the notices had been run into one
  squeezed line), and to the left the same day, when the owner put the
  community's Discord invitation in the top-right corner as a tile of the
  same shape (no notice the policies ask for; it is hidden below Wide):
  the same sentences, all on screen on every face, none a press away. The build
  stands under the text row on every class (`door::version_line`); it is
  no notice the policies ask for.
- **Narrow, Compact and Phone**: the **one-line colophon** on every face —
  **"Unofficial Fan Content"** (which opens About), "Card data and images
  by Scryfall", and **"Source (AGPL-3.0)"** (which opens the address); the
  build stands under the text row — and the **About** sheet, one press away from every face (the
  text row's About, and the colophon's notice), with the notice word for
  word, Scryfall's credit, the licence, the source address as text and
  link, its code, and the third-party licences.

Checked against the
[Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy),
reread for this change: "Tell the Community it's unofficial. Make it clear
that your Fan Content is not endorsed or sponsored by Wizards—i.e.,
unofficial." Every face says "Unofficial Fan Content" at every size; the
full notice the policy gives is in the client on every face, one press
away where a phone's height has no room for its three sentences. This is
the grey-area record AGENTS.md asks for: the policy's own notice is not on
every *screen* of a phone face, only one press from it. The owner approved
the design that does this (06.10.2026, DESIGN-v5 Q12); the PM routes this
entry to the owner for the explicit okay on the wording above. Scryfall's
attribution ("data and images provided by Scryfall") and the AGPL §13
offer (an "opportunity to receive the Corresponding Source" for every
user interacting remotely) are met on every face: the credit is on the
line itself, the offer is a press on the same line. The third-party
licences (Alegreya Sans, Faustina, Font Awesome, Mana — OFL 1.1; MTGJSON
— MIT; the Rust crates, under their own) are now named in the client
itself (About, N4-8), not only in `NOTICE`.
