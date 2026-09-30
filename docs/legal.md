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
   `crates/baylee-client/src/sound.rs`. The continuous music is an original
   composition, now performed using recordings of real instruments from
   **Versilian Studios VSCO 2 Community Edition**, not the paid editions.
   The [publisher's original-WAV page](https://versilian-studios.com/vsco-community/)
   states: “Licensed under CC0 (Creative Commons Zero) you can do whatever
   you want with these samples.” Checked 26.09.2026, together with the
   [source repository licence](https://github.com/sgossner/VSCO-2-CE/blob/440300901dfe9275fd84e0b7763af1f8443ae62e/LICENSE).
   The full CC0 dedication is shipped in
   `crates/baylee-client-core/assets/orchestra/LICENSE-CC0.txt`.
   Recordings: Sam Gossner and Simon Dalzell; sample editing: Elan Hickler /
   Soundemote. `art/music/samples.json` records each original path, pinned
   revision, SHA-256 and prepared PCM hash. `art/music/prepare.py` reproducibly
   downmixes/resamples the selected recordings to mono 22,050 Hz PCM16.
   The score and sampler are project code under AGPL; the samples remain CC0.
   The owner's follow-up removes flute and adds fifteen `Keys/Upright Nr1`
   recordings from the same pinned CC0 source: five registers, three dynamics.
   The publisher licence quote above was checked again for this revision.
   No game soundtrack, melody or recording from Wizards or Blizzard is used.
   This replaces the earlier arithmetic-only music decision at the owner's
   explicit request for real orchestral instruments on 26.09.2026.
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
   (`lobby::front::source_address`). Since #299 it is a link that opens
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
