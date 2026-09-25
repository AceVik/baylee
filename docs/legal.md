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
   the seat mats, the lobby's backdrop, the sky and **the weather over the
   table** are all computed rather than shipped
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
   attribution in clients. The cache is for our own players and never a
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
     it, and only its dome lies over it
     (`shellui::tests::every_colour_but_the_domes_carries_the_mask`).
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
   The lobby's music (#296) is the same answer at the length of a tune.
   `baylee_client_core::music` holds a composition written for this client,
   note by note, and synthesises it as it plays: square, pulse and triangle
   voices and a frame drum made of sines, in the manner of the demo scene's
   cracktros and trainers. No module file, sample or recording from the
   scene or anywhere else is in it, and it quotes no known melody: no keygen,
   cracktro, trainer, game or film theme, and nothing of Wizards' (the Fan
   Content Policy: "Don't use Wizards' Video or Music in your Fan Content").
   The style is borrowed, which costs nothing; the notes are ours.
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
10. **A front door after someone else's (#295).** The gateway and sign-in
    faces stand in a scene (`crates/baylee-client/src/vista.rs`,
    `shaders/vista.wgsl`) that the owner asked to have the depth of the
    login screen of Blizzard's *World of Warcraft: Midnight*. What came
    from it is a mood and a job: a cool foreground frame around a warm,
    bright world the form stands in, a lit seam where the two meet, depth
    told in parallax, and joining told as passing a threshold. Nothing of
    its expression is here: no round ornate ring, no spikes standing off
    it, no jewels at its sides, no crystal wings, no city, tree or figure,
    and no Blizzard image, texture, logo, font, layout or motif; nothing
    was traced or sampled, and the screenshot that showed the mood was
    deleted once the scene was done. Ours is a geode: the screen is the cut
    face of a dark mineral, agate bands following the cavity's line out
    into rough rock; the opening is its cavity, a broken superellipse that
    is nowhere a circle, lined with crystal teeth that point inward as a
    geode's do; and the world seen through it is a first light over a
    ridge of crystal fins, mirrored in a resin floor. It is arithmetic,
    like clause 5's music and the table's felt, it has no Magic art, and
    its colours are the lobby's own.

The table UI icon map (`baylee-client-core/src/tableicons.rs`) was checked on
17.09.2026 against the same policy table and the upstream Mana 1.18 stylesheet
(https://github.com/andrewgioia/mana/blob/master/css/mana.css). It uses zone,
untap, sorcery, combat/ability and counter marks, with generic end/cleanup
controls from Font Awesome. It adds no logos or faction watermarks.
