# Legal & Licensing Policy

Private, non-commercial fan project hosted as open source on GitHub.
Not legal advice; for a public/commercial launch consult a Fachanwalt für
Urheber- und Medienrecht.

Central login review (19 October 2026): the [Wizards Fan Content Policy and
FAQ](https://company.wizards.com/en/legal/fancontentpolicy) was checked for the
new account-first native screen. Its access rule says: "You can’t require
payments, surveys, downloads, subscriptions, or email registration to access
your Fan Content;". The FAQ separately permits sign-in for private group play.
The implementation asks for a username/password, no email or payment, and keeps
offline access available. This is an account-service UI change, with no new
card art, symbols, fonts, music or other third-party assets. Existing asset and
Scryfall requirements below still apply; the API page returned HTTP 403 during
this check, so no claim of newly verified Scryfall wording is made here.

Desktop feedback review (25 October 2026, PRs #395/#396): the
[Wizards Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy)
was checked, including “Tell the Community it’s unofficial.” The retained
headers keep the existing notice and access to About. The changes use existing
licensed icon fonts and procedural surfaces; no third-party assets or corpus
files were added. The Zombie token UUID resolves through Scryfall to the black
2/2 Zombie from tm20, collector number 6. Scryfall’s API documentation returned
HTTP 403 during this review; the existing image terms and recorded decisions
below remain the basis for runtime card images and finishes.

## Guardrails for every change

- Unofficial non-commercial fan project. No WotC assets in the repo, except
  the card images inside recordings of the game (§3, "Recordings of the game
  show its card images").
- The card-script reference is an external, GPL-licensed corpus that codegen
  may read as an automated lookup from a local checkout. No file of it is
  ever copied into this repository or distributed with it. `NOTICE` names it
  exactly and is the one place that does; everywhere else it is "the corpus".
- Every change to cards, icons, symbols, card images, fonts, audio or
  anything else shown to a player is checked against the rules we depend on:
  the Wizards Fan Content Policy and its FAQ, Scryfall's API and image
  terms, and the licences of what we ship (`docs/legal.md`, #270). Quote the
  rule from its page, never from memory.
- A grey area (the rule's wording is met loosely or not at all, but the
  practice is tolerated) is allowed only with the owner's explicit okay,
  recorded in `docs/legal.md` with the rule quoted beside it. The accepted
  ones so far: the Mana font's mana and tap symbols (§2a), foil over a
  card image, and a passing state over a card image while it lasts (dimmed
  while a question leaves the card out, phased out, asleep; the owner,
  13.10.2026, §3: the image rule is about lasting alteration).
- Anything **clearly forbidden** is never built, merged or shipped quietly.
  Stop and tell the owner in plain words, with the rule quoted verbatim and
  the file and line that break it. A session tells the PM, and the PM tells
  the owner. This applies even when the change was requested and even when
  it looks small.

## The rules we depend on

1. **Code license:** AGPL-3.0-only (`LICENSE`). `NOTICE` carries the fan
   content disclaimer.
2. **WotC Fan Content Policy:** the service is and stays completely free
   (no paywall, no paid features), and asks for no e-mail address, which
   the policy names beside payment as a price fan content may not charge:
   an account is Baylee Central's, a username and a password, with no
   address asked for, and a guest is a display name and nothing else
   (#269; `docs/protocol.md` §"Usernames", §"Playing as a guest"). Since
   30.10.2026 each gateway's operator chooses whom it lets in: guests,
   Central's accounts (the default) or both (`BAYLEE_AUTH_MODE`,
   `docs/central-login.md` §"Who a gateway lets in"), so guests are no
   longer offered everywhere; offline play needs neither, and Central asks
   for no address (the Fan Content Policy check of 30.10.2026 found guests
   not required). A guest may not upload a sleeve
   or a mat: an account anybody gets by asking cannot answer for a picture
   put on the gateway, so only an account may; clients show "unofficial fan content,
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

   **The landing page's card status shows card pictures the same way**
   (12.10.2026; narrowed after a legal review, 13.10.2026). `web/site`
   lists every set and, when a visitor opens one, that set's cards with
   their status; the visitor's browser loads each picture from
   `cards.scryfall.io` (`small` in the grid, `normal` when a card is shown
   large, `loading="lazy"`), the URL built from a printing id alone. The
   page and its server never fetch, store or pass on an image, the page
   never calls Scryfall's API (its CSP: `img-src` names that one foreign
   host, `connect-src` only `'self'`, GitHub's API and our feedback
   service; `scripts/server/site.caddy`), its referrer policy is
   `no-referrer`, and nothing lies on a picture: it is shown whole
   (`object-fit: contain`), never filtered, faded or tinted whatever the
   card's status, which only the badge under it says
   (`baylee-tools site checks` holds the CSS to that). Each card is a
   link to its page on Scryfall, where its artist and printing are. Each
   opened set says "Images: Scryfall · © Wizards of the Coast", the set
   list says the images are © Wizards of the Coast and shown unaltered from
   Scryfall, and the footer carries the Fan Content Policy notice in
   Wizards' English and official German wording, linked to the policy, a
   rights and trademark line, and "Card data and images provided by
   Scryfall". What the page publishes itself is the ledger's own facts
   (name, set) plus our status, the Scryfall identifiers needed to show
   and link a picture (`oracle_id`, a printing id) and Scryfall's set
   names; no card text, price or other Scryfall field. Pictures are shown
   only for cards in the pool and never for Universes Beyond cards, whose
   properties are not Wizards' to permit: `crates/baylee-tools/src/site/card_status.rs`
   writes a printing id only for a card in the pool (or on the never-built
   list) whose printing has no triangle security stamp and is not of a
   crossover set (`UB_SETS`; the newer Marvel and Avatar sets carry no
   triangle, so the stamp alone does not catch them, measured 13.10.2026);
   every other card is listed by name. Unlike the admin console the page is
   public, so it rests on the same reading as the client's art: the page
   never fetches, stores, caches or passes on a picture, so it neither
   republishes nor proxies Scryfall's data ("You may not simply repackage,
   republish, or proxy Scryfall data"); each picture is shown whole and
   credited, on a free page. A public page is a new surface: the PM routes
   this entry to the owner for the okay.

   **Recordings of the game show its card images** (owner, 23.10.2026).
   The README's loops and the website's screenshot section are recordings
   of the client as it plays, card images included; until then they used the
   text view. The rule above, "No card images are committed to the repo",
   was written against shipping an image as an asset (the card back); a
   recording of the game is community content, which both rules name:
   Wizards' Fan Content Policy ("You can use Wizards' IP (except for the
   restrictions listed in #3) to make Fan Content"; Wizards IP includes
   "cards, … pictures, … artwork", and Fan Content "fan art, videos, …
   websites"; it excludes only "the verbatim copying and reposting of
   Wizards' IP") and Scryfall ("Scryfall provides our card data and image
   database free of charge for the primary purpose of creating additional
   Magic software, performing research, or creating community content (such
   as videos, streams, podcasts, etc.)"; its image page calls the PNG "the
   best image to use for videos or other high-quality content"), both read
   on 23.10.2026. The conditions, each from those pages: a recording shows
   cards as the client draws them, scaled uniformly and never in a passing
   dimmed or tinted state ("Do not blur, sharpen, desaturate, or color-shift
   card images", "Do not distort, skew, or stretch card images"), with
   framings that keep each card's artist and copyright line ("Do not cover,
   crop, or clip off the copyright or artist name on card images"), and
   nothing of ours on a card ("Do not add your own watermarks, stamps, or
   logos to card images"); no Universes Beyond card appears (above: their
   properties are not Wizards' to permit); every place that shows them
   credits "Card images: Scryfall · © Wizards of the Coast" next to them and
   stays free with the unofficial-fan-content notice ("Portions of the
   materials used are property of Wizards of the Coast."); and
   `docs/images/readme/` and `web/site/img/` say that the card images in
   their files are Wizards of the Coast's, provided by Scryfall, and not
   under this repository's licence. No card image is committed as an asset
   of its own.

   **Nothing we paint lies on a card image** (#274). Scryfall asks that an
   image is not covered, cropped, blurred, sharpened, desaturated, colour-shifted
   or stamped, and the artist's name
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
   - the **shell** of a protected permanent: since 13.10.2026 a slim
     animated **border** standing round the card from a hair above its face
     down to the felt, one layer each for indestructible, defender and
     hexproof or shroud (the owner replaced #298's steel rim, glass domes
     and brick wall with it), and summoning sickness's wave of moonlight a
     hair over the face. Two rules hold them, by the owner's and the PM's
     decisions under #298. Over its own print the border and the ring it
     lies down to are exactly transparent: a mask follows the real camera's
     ray through each of their points to the card's face
     (`shellmat::the_mask_is_exactly_zero_over_the_print`, and
     `every_colour_but_the_waves_carries_the_mask` holds every other return
     to it); a keyword lasts as long as the keyword, so it is no passing
     state (below). **The wave is the one exception, by the owner's okay of
     25.09:** it passes over the print and leaves nothing behind, as the
     arrival sweep does, and where no crest is it draws nothing. No shell
     ever lands on another card's print: where a border standing round its
     card would reach another card's face this frame it lies down on the
     felt as a ring, and the wave is not drawn
     (`table::shell_tests::a_standing_border_never_lands_on_another_cards_print`
     and `a_wave_never_lands_on_another_cards_print`, swept over duels and
     rings, fanned rows, fliers, hovers and every seat's camera). The hover
     preview wears the same shells, standing, and holds the same rule: its
     border is exactly transparent over the preview's print, the mask being
     the card's own outline seen from straight over it, and only the wave
     lies over it (`shellui::tests::every_colour_but_the_waves_carries_the_mask`).
   What reaches the image itself: its own **finish** (a foil is what that printing is, the one
   exception the owner accepted) and light that passes over the whole card
   and leaves nothing behind — the table's lamp pool, the one-second arrival
   sweep and the zone-change doors. The brushed coating every card used to
   wear lifted the image's blacks, the artist's line included, and was taken
   off. `cardmat::nothing_but_the_finish_is_drawn_on_the_print` holds both
   card shaders to that; `docs/client.md` §"The print fills the card" has
   the rest.

   **Owner decision (13.10.2026): the rule is read as being about lasting
   alteration.** Scryfall's guideline says "Do not blur, sharpen,
   desaturate, or color-shift card images", and draws no line between a
   lasting change and a passing one. The owner reads it as forbidding a
   change to the image for good, and decided that a *passing* visual state
   may lie on the print for as long as it lasts ("gilt nur für permanent,
   für vorübergehend ist es voll okay"). That reading and the risk in it
   are his decision, not something Scryfall has said. Three states do, each ending with the game state that
   causes it and leaving the print exactly as it was: a card the question
   this seat is answering will not take is **dimmed** until it is answered;
   a **phased-out** permanent is drawn grey and faded until it phases in
   (CR 702.26b); a creature under **summoning sickness** wears a cool
   moonlit edge until it may attack (CR 302.6). They are one term in each
   card shader, weighed by the card's own `veil` and zero on every other
   card (`cardmat::veil`, `card_common.wesl`'s `veiled`); the guard test
   above allows that one term and no other, so a lasting stamp, a keyword
   for instance, is still refused there. A keyword lasts as long as the
   keyword, so its border stays off the print (the shell's mask, above).

   **A language-model seat asks Scryfall's API only for a profile that
   says so** (31.10.2026, `docs/llm-seat.md` §"Reports and lookups"). A
   seat's card lookups read this build's compiled Oracle first and its
   gateway's catalog under its own session second; only a profile with
   `"lookup": "scryfall"` asks `api.scryfall.com`, for a card's rulings and
   for a card neither knows. It keeps the requirements recorded above: at
   most ten requests a second (one at a time, 100 ms apart within a
   process), what it fetched kept (a week, in the user's cache directory,
   for this machine's seats alone: no mirror, no proxy), and it names
   itself (`User-Agent: baylee-seat/<version>`, `Accept:
   application/json;q=0.9,*/*;q=0.8`). A request carries the card's name
   and nothing about the game or the player; the rulings' address is
   followed only on `api.scryfall.com`. The text a model is told names
   Scryfall as its source. Scryfall's API documentation answered HTTP 403
   to this check as well, so no newly verified wording is claimed here;
   the headers are the project's brief.
4. **Privacy:** self-hosted; minimal account data; account deletion
   endpoint (`DELETE /account`, `docs/protocol.md` §"Deleting an account
   (#292)"); no tracking.

   **The website's privacy notice** (13.10.2026, after a legal review):
   `web/site/datenschutz.html`, in German and English, linked from every
   page's footer and beside the contact form. It names the controller only
   by pointing to the Impressum; says that the server is rented from netcup
   in Germany and that the site writes no access log
   (`scripts/server/site.caddy`); that there are no cookies and no
   tracking, only the language choice in `localStorage` and GitHub's
   answers in `sessionStorage` (§ 25(2) no. 2 TDDDG); that the visitor's
   browser itself asks GitHub's API and, for an opened set or a card shown
   large, Scryfall's image server, both in the USA; what the contact form
   and the Impressum's protection keep and for how long (as
   `docs/privacy.md`); the legal bases (Art. 6(1)(f), the contact form also
   (b)); the data subject's rights, the complaint to a supervisory
   authority (the LDI NRW for the owner); and that no automated decision
   is made. **A draft for the owner's lawyer**: in particular whether a
   processing agreement with netcup is needed and in place, how long the
   server's system journal keeps what it keeps, and the transfers to the
   USA (GitHub, Scryfall) are for them to confirm.

   **Impressum (owner decision, 13.10.2026).** The landing page carries one
   (`web/site/impressum.html`, linked from the footer and the contact
   section): "Angaben gemäß § 5 DDG" for a private, non-commercial project
   of a private individual, with name, postal address, e-mail address and
   telephone number, and the contact form as an additional channel. No
   company, VAT, register or profession lines, because there are none. No
   "Verantwortlich nach § 18 Abs. 2 MStV": the site is a project page
   (what Baylee is, downloads, card status, release data), not a
   journalistic-editorial offer. No ODR or consumer-arbitration block (no
   contracts with consumers); one neutral sentence says so. The owner's
   details are in no file of the repository: they live in a private JSON
   on the owner's machine, `crates/baylee-tools/src/site/impressum.rs` turns them into
   an obfuscated blob that only the server holds, and the page's script
   renders them at runtime; robots.txt, `X-Robots-Tag` and a same-origin
   check keep crawlers off (`web/site/README.md` §"The Impressum").
   Every field is drawn character by character in a shuffled DOM order
   among invisible decoys (owner, 13.10.2026), so whatever reads the DOM as
   text (textContent, innerText, the accessibility tree) gets a jumble, and
   the page holds no `mailto:`/`tel:` link: writing, calling and copying
   happen in a click handler. The details load only when the visitor presses
   *Impressum anzeigen* and the browser has solved a proof of work of about
   a second, handed out by the feedback service, which also keeps a
   honeypot (`docs/feedback.md` §"The Impressum: `/site/impressum`").
   **The rule (owner, 13.10.2026): no person is ever locked out.** The
   Impressum must stay easy to reach (one click from every page, the
   details one click more), so nothing refuses a person: a request that
   looks like a robot gets a harder puzzle (at most 16 times the work),
   and only a robot beyond doubt, or an address that followed the
   honeypot's invisible link, gets invented details; the contact form and
   the no-JavaScript sentence stay as the other way. There is no hard
   block: the owner decided against fail2ban (13.10.2026), and the files
   for it were removed.
   **Trade-off, accepted by the owner:** the
   drawn block is `aria-hidden`, so a screen reader cannot read the address
   either; a sentence only screen readers hear says so and points to the
   copy buttons (which put the right text on the clipboard) and to the
   contact form.
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
   its pitch and loop facts; `baylee-tools art prepare` rebuilds the bank
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
Power Surge would add another rules implementation. The legal guardrail (§"Guardrails for every change")
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

The resumed original-card batches 51 and 55 (2026-10-15) continue under this
owner decision. The [official policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were re-read: “Tell the Community it’s unofficial.” remains covered by the
existing notices. Rules and behavioral tests introduce no artwork, music,
font or corpus source files. The Scryfall API terms endpoint returned HTTP 403
during this check; no fresh successful read of that page is claimed, and the
existing API/image obligations remain in force.


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
by `baylee-tools installers art` and committed: the dmg window's
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
`baylee-tools installers art` to a rounded square whose corners are a
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
the grey-area record §"Guardrails for every change" asks for: the policy's own notice is not on
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


### Performance and partial-card completion review (2026-10-11)

The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were re-read for this work; the owner decision of 2026-10-02 above remains the
basis for rules implementation. This work changes executable rules, tests and
rendering workload without adding artwork, fonts, audio or corpus files.
The [Scryfall API terms](https://scryfall.com/docs/api) were also re-read:
“You may not simply repackage, republish, or proxy Scryfall data.”
The existing attribution, authenticated catalog and original card-image path
remain in place. Rendering fewer offscreen cards does not change their images.

The completion of Mage Siege adds the central 0/1 black Wizard token definition
and its verified [TFIN #14 identifier](https://scryfall.com/card/tfin/14/wizard).
Only the identifier and independently implemented rules enter the repository.
The existing client image-fetch path and attribution apply; the image-integrity
rule quoted above from the [Scryfall API terms](https://scryfall.com/docs/api)
continues to apply to this token too. No image or corpus file is distributed.

The next completion batch registers the verified [TMH3 Servo](https://scryfall.com/card/tmh3/33/servo),
[TFRC Phyrexian Mite](https://scryfall.com/card/tfrc/16/phyrexian-mite), and
[TFDC Phyrexian Germ](https://scryfall.com/card/tfdc/4/phyrexian-germ) identifiers.
Their characteristics and abilities were checked against those API printings.
The existing Scryfall attribution and original image-fetch path remain in use;
no artwork is committed. Serpent Generator's colorless artifact Snake has no
matching token printing in the checked catalog, so it has no image identifier
and uses the existing rules-face fallback. It never borrows an image depicting
a different Snake's colors, types or abilities.

Batch 26 completes four existing card definitions and corrects the death-return
path of two others without adding media, printing identifiers or corpus files.
The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were checked again; “Don’t mess with the legal notices in our stuff.” continues
to apply. Attribution and the existing original-image path are retained. Rules
implementation continues under the explicit owner decision above, without a
claim of new permission from Wizards. No additional asset licence is needed.

Batch 27 implements the existing Starting Town, Halfdane, Hazezon Tamar and
Giant Slug definitions using the current Comprehensive Rules, including
603.7b-e for delayed triggers and 612.1/612.3 for text and granted abilities.
It adds no media, printing identifiers, copied scripts or external assets;
the policy review and owner decision recorded above continue to apply.

Batch 28 completes Field of Dreams, Revelation, Dream Coat and Alchor's Tomb.
Dream Coat's permanent color change and allowed combinations were checked
against the official rulings served by [Scryfall](https://api.scryfall.com/cards/07edbbf4-c3d6-4ec1-ae9b-4ae202fb6998/rulings),
and against CR 105.4, 303.4m and 608.2h. Public hand browsing uses the existing
card text and image paths; this batch introduces no assets or copied scripts.
The policy review and explicit owner decision above continue to apply.

Batch 29 completes Urza, Lord High Artificer, Cori Mountain Monastery and Nivix,
Aerie of the Firemind. Nivix's duration and normal payment/timing restrictions
were checked against the official rulings served by
[Scryfall](https://api.scryfall.com/cards/a025a42d-cbcc-4bd2-9331-5c7dc677da20/rulings).
The current Comprehensive Rules supply 400.7, 514.2, 601.3e and 614.10a for exact
card identity, cleanup, alternate spell characteristics and skipped turns.
No media, printing identifiers or corpus scripts are added. The existing policy
review and explicit owner decision continue to apply.

Batch 30 completes Birgi, God of Storytelling // Harnfel, Horn of Bounty.
The [official rulings](https://api.scryfall.com/cards/44657ab1-0a6a-4a5f-9688-86f239083821/rulings)
confirm that Birgi's trigger uses the stack, boasting still requires attacking,
and Harnfel grants ordinary play permissions independent of its source's later
presence or controller. CR 602.5b, 702.142a and 514.2 govern the activation tally,
boast restriction and mana-retention lifetime. This uses the existing card
printing and adds no assets or external scripts; the policy review and owner
decision recorded above continue to apply.

Batch 31 completes Planar Nexus, Nearby Planet, Teferi's Isle, The Grey Havens,
Plaza of Heroes and Skycoach Waypoint using existing printings. The official
[Nexus rulings](https://api.scryfall.com/cards/28603c1c-f9b4-4001-bc56-d1453d5cacf5/rulings)
confirm its interaction with the Urza lands. The
[Nearby Planet rulings](https://api.scryfall.com/cards/da785d1b-6b90-4b65-9efb-d7f329405318/rulings)
state: “Nearby Planet can't be tapped for {C}.” Its five basic land subtypes
supply the corresponding colored mana abilities. CR 604.3, 502.1 and 722.3a
were checked in the current Comprehensive Rules for CDAs, simultaneous phasing
and preparation. This batch adds no assets, printing identifiers or corpus
scripts; the existing policy review and explicit owner decision remain applicable.

The preparation correction follows CR 722.2b, 722.3 and 722.3c in the
[2026-09-25 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt).
Prepare characteristics are copiable, the physical card cannot be cast using
its inset characteristics, and the designated permanent creates a persistent
copy in exile. Fatehold Chronologist / Peer Review and Emeritus of Woe keep
their existing printings; this correction adds no art, external scripts or
other assets. The policy review and owner decision above remain applicable.

Batch 32 completes Hall of Tagsin, The Great Mound, Junktown and Sokenzan,
Crucible of Defiance. Token characteristics were checked against the official
Scryfall records for [Powerstone](https://api.scryfall.com/cards/d45fe4b6-aeaf-4f84-b660-c7b482ed8512),
[Vibranium](https://api.scryfall.com/cards/7f9e9b3a-c515-449a-95da-ee3f5175b74f),
[Junk](https://api.scryfall.com/cards/62c4a6f6-6425-4c0c-b35a-880fcab42aad) and the
[ordinary Spirit](https://api.scryfall.com/cards/c672ee51-0e77-463f-8586-191b413c44dd).
CR 111.10w defines Vibranium; CR 205.3g includes Heartwood, Lander, Mutagen and
Vibranium, which the current Scryfall artifact-type catalog omits. Codegen
supplements those names without modifying the cached API response or reassigning
any existing subtype ID. Only rules data and existing printing identifiers are
added; no images, audio or corpus scripts are copied. The Fan Content Policy
review, Scryfall terms and explicit owner decision recorded above apply.

Batch 33 completes Country Roads, Reef Roads, Rocky Roads, Foul Roads, Wild
Roads and Realm of Koh. The token definitions use the verified Scryfall records
for the [Aetherdrift Pilot](https://api.scryfall.com/cards/8672d795-04f9-4089-9c92-6d6ff628da12)
and [Realm of Koh Spirit](https://api.scryfall.com/cards/f59eba51-458a-40e0-b754-999f91d5d839).
Crew and saddle follow CR 702.122a-b and 702.171a-c in the
[current Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt),
including the noncopiable, temporary saddled designation and payment-only
Pilot power bonus. Only rules data and existing printing identifiers are added;
no artwork or external scripts are copied. The policy review, Scryfall terms
and explicit owner decision above remain applicable.

Batch 34 completes the existing Ur-Drago, Lord Magnus and Detection Tower
printings. CR 609.4 in the
[current Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
states: “This applies only to the stated effect.” Their as-though permissions
therefore leave the landwalk and hexproof abilities intact. CR 702.11b-c and
702.14c define the affected targeting and blocking restrictions. No assets,
printing identifiers or external scripts are added; the policy review and
explicit owner decision recorded above continue to apply.

### Counter rules and completed cards, batch 35 (2026-10-11)

The eight card implementations add original rules code and retain their
existing Oracle/Scryfall headers; no new art, recordings, fonts or corpus
source are shipped. The existing fan-project authorization and Scryfall
data handling documented above continue to apply.

The primary [Comprehensive Rules, 2026-09-25](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
say in CR 613.7c: “Each counter receives a timestamp as it’s put on an object or player.”
Keyword counters therefore enter the normal layer ordering (122.1b), hone
counters apply independently of Equipment abilities (122.1j), and counter
limits apply both to placement and to excess-counter state-based actions
(122.4, 704.5r). Source and attachment last-known information follows 608.2h;
returning permanents retain no previous incarnation's turn-start marker
(400.7). The source file was inspected locally against that published edition.

Batch 36 completes the existing Mikaeus, the Unhallowed, Nim Deathmantle and
Borg Queen, Perfection Manifest printings using original engine code. The
existing policy review and owner authorization above cover these unchanged
headers and identifiers; no artwork, recordings or corpus code are added.
The [current Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
provide the rules basis: 603.10a for pre-departure abilities, 702.93a and
702.79a for return-keyword counter conditions, 400.7 for incarnation identity,
and 611.2e for characteristic changes applied simultaneously with entry.

Batch 37 completes the existing Fire // Ice, Pyrotechnics and Shatterskull
Smashing printings with original rules code and unchanged identifying
headers. No new assets or corpus source are distributed; the existing
policy review and owner authorization above still apply.
The primary [Zendikar Rising release notes](https://magic.wizards.com/en/news/feature/zendikar-rising-release-notes-2020-09-10)
state: “If X is 6 or more, you divide twice X damage”. Accordingly, the
engine announces the doubled total before costs, preserves each chosen
share through copying and retargeting, and never redistributes an illegal
target's lost share. CR 601.2d, 603.3d, 608.2b and 707.10 govern the shared
implementation, checked against the current published rules cited above.

Batch 38 completes the existing Brain Freeze, Flusterstorm and Storm-Kiln
Artist printings with original rules code and unchanged identifying headers.
No assets or corpus code are added; the existing policy review and owner
permission above still cover this work. The primary
[Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt),
checked in the published text, define the storm cast trigger and earlier-spell
count in 702.40a, and distinguish copying from casting in 707.10. Each copy
retains its original decisions and offers independent target changes.

Batch 39 completes the existing nine discard/connive/recruit printings with
original rules code and their identifying headers retained. The existing fan
policy review and recorded owner permission above still apply. The only added
presentation metadata is the identifier for the matching white 1/1 Human Soldier,
[THOB token 2](https://scryfall.com/card/thob/2/human-soldier), verified against
Scryfall's API on 2026-10-11. No image or corpus source is added to the repository.
The [published Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
state in 701.50e: “If a permanent would connive 0, no connive event occurs.”
Sections 701.50a–f and 701.70a govern connive and recruit, 701.9c governs undefined
characteristics of unrevealed hidden-zone discards, 113.6k allows hand discard
triggers, and 101.4 governs simultaneous choices and actions. These sections were
read in the published text rather than inferred from reminder text.

Batch 40's counter foundation extends existing card printings with original
rules code and retains their identifying headers. The fan-policy review and
recorded owner permission above continue to apply; no art, font, audio or corpus
source is introduced. Counter behaviour was checked against the
[published Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt):
122.6a assigns the placing player, 701.34a defines proliferate, and 614.16 limits
replacements worded around effects to the relevant placement events. The
[Modern Horizons 3 release notes](https://magic.wizards.com/en/news/feature/modern-horizons-3-release-notes)
confirm that energy is a persistent player counter resource and that a payment
requires the specified number of counters.


Batch 45's rules work was reviewed on 2026-10-12 against the
[Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
and [Scryfall API and image terms](https://scryfall.com/docs/api). The former
requires: “Don’t mess with the legal notices in our stuff.” The latter states:
“You may not simply repackage, republish, or proxy Scryfall data.” Existing
attribution and the original card-image path remain intact. The recorded owner
decision above continues to govern independent rules implementation. This batch
adds no artwork, audio, fonts, or corpus source. Its Jace token has an original
rules definition and uses the existing text fallback without an image identifier.
Rules were checked against the published September 25, 2026 Comprehensive Rules
and the cards' Wizards rulings served through Scryfall, including the distinct
requirements for casting card copies, escape, entry counters and outside-game
choices. Scryfall's documentation returned HTTP 403 to the web reader; the same
page was successfully retrieved directly before this review was recorded.

Batch 46's in-progress implementation was reviewed on 2026-10-12 against the
[Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
and [Scryfall API and image terms](https://scryfall.com/docs/api). Wizards states:
“Don’t mess with the legal notices in our stuff.” Scryfall requires:
“Your software must create additional value for end-users.” The existing owner
permission recorded above continues to govern independent rules implementation.
The new printing facts describe rule-relevant watermarks, silver borders and
acorn stamps; they introduce no artwork or copied corpus implementation.
Cryptic Spires' circles are player choices outside the unmodified card image.
Scryfall's page returned HTTP 403 to the web reader but its official HTML was
successfully read directly; quotes were checked against that response.

The [Lost Caverns of Ixalan release notes](https://magic.wizards.com/en/news/feature/the-lost-caverns-of-ixalan-release-notes)
confirm that Myriad Pools copies casting decisions, retains the recipient's
counters and other noncopy effects, and causes no new battlefield entry.
The implementation therefore retains exact original spell incarnations for
later copies and separately restores temporary copied rules and decisions.


### Speed display and generated-card test pilot (2026-10-15)

The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy) were reread: “Tell the Community it’s unofficial.” Existing unofficial notices, attribution and the owner decision on rules implementation above remain in place. The Raceway implementation and Destroy Evil / Nature's Claim tests introduce no copied implementation, imagery, recordings or fonts. The speed display uses ordinary text and numerals in the existing licensed font. Scryfall's API documentation returned HTTP 403 during this check; cached card data were used, and no new API consumption or image handling is introduced.

- 2026-10-15, 32-card Oracle-blind test trial: behavioral tests and generated rule bindings only; no new card images, symbols, audio, or corpus source are distributed. The existing owner-approved rules-implementation exception remains applicable. The Fan Content Policy requirement “Tell the Community it’s unofficial.” remains satisfied by the existing unofficial fan-project notice (https://company.wizards.com/en/legal/fancontentpolicy). Agony Warp's independent-target behavior was checked against Wizards' Modern Masters 2015 release notes (https://magic.wizards.com/en/news/feature/modern-masters-2015-edition-release-notes-2015-05-12); corpus lookup stays external and generation remains through xtask.

### 2026-10-18 — Completing mutate, venture and Hanweir meld

Independent rules code completes Gemrazer, Dungeon Descent and Hanweir Battlements,
and adds Hanweir Garrison with its identifying header. No corpus implementation,
card image, font, icon or audio is copied into the repository. Dungeon room names
and choices use the existing text renderer; card images use the existing Scryfall
pipeline without alteration. The owner-approved rules-implementation decision
above continues to apply.

The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were checked: “Tell the Community it’s unofficial.” Existing notices and credits
remain in place. The [Scryfall API/image terms](https://scryfall.com/docs/api)
returned HTTP 403 to both the web reader and direct HTTP access during this check;
that access failure is recorded rather than presented as a fresh verification of
those terms. Existing Scryfall licensing decisions and delivery restrictions above
remain applicable; no new distribution or image treatment is introduced.

### Lobby discovery refinement (18.10.2026)

This change draws original UI geometry, text, borders and transitions with the
existing licensed font stack. No new art, symbol, font, audio or card-image
asset is introduced. The [Wizards Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were checked: “Tell the Community it’s unofficial.” and “Don't mess with the
legal notices in our stuff.” The existing disclaimer/source links and card-image
pipeline remain in place; existing Scryfall image and font licence checks above
continue to apply.

### 2026-10-19 — Tour visibility at fractional Windows scale

The tour fix changes placement of our own UI bubble and adds development
diagnostics. Existing fonts, artwork, card images and notices are unchanged;
no recorded system audio is shipped. The
[Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were reread: “Tell the Community it’s unofficial.” The existing notice remains.
The [Scryfall API/image terms](https://scryfall.com/docs/api) returned HTTP 403
in this check; no new image usage or distribution is introduced, and the
existing decisions above continue to apply.

### 2026-10-24 — Shared translucent navbar and compact deck controls

The navbar and deck controls use original UI geometry and the existing
unmodified Font Awesome font for action icons, under the bundled SIL OFL 1.1
(`crates/baylee-client/assets/fonts/licenses/OFL-FontAwesome.txt`, `NOTICE`).
No artwork, card-image treatment, corpus source or font asset is added.
The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were reread: “Tell the Community it’s unofficial.” Existing notices and credits
remain. The [Scryfall API/image terms](https://scryfall.com/docs/api) returned
HTTP 403 during this check; this change introduces no new image or API usage,
and the existing licensing decisions above remain applicable.

### Baylee memorial · 26 October 2026

The owner explicitly requested freely usable repertoire and a memorial album
using his five supplied cat photographs. The revised menu performance uses
complete **public-domain Mutopia editions** of Debussy's *Clair de lune*
(Keith OHara, after E. Fromont 1905) and Satie's *Gymnopédie No. 1* (Evin
Robertson). Edition pages and source hashes are in `art/music/memorial/`.
The [Debussy edition](https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=1778),
[Satie edition](https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=37), and
[Mutopia terms](https://www.mutopiaproject.org/legal.html) identify these
particular editions as public domain; checked 26.10.2026. This is the owner's
documented exception to the original-only tune rule. The earlier OpenScore
Lieder arrangement and its source files have been removed. No Yiruma melody
or recording is used; the reference described the desired mood.

New grand-piano samples are **Osiris Piano**, Versilian Studios/Karoryfer,
[Yamaha C2, CC0](https://shop.karoryfer.com/pages/free-osiris-piano). The
[publisher's sample policy](https://shop.karoryfer.com/pages/free-samples)
confirms CC0; checked 26.10.2026. Files are pinned to
`sfzinstruments/Osiris_Piano@18c6afccb60cff458edbf7c394571783e074e1e9`.
Full stereo solo-violin/cello recordings use the existing approved
`sgossner/VSCO-2-CE@440300901dfe9275fd84e0b7763af1f8443ae62e` CC0 collection;
its [publisher page](https://versilian-studios.com/vsco-community/) was rechecked.
Both unmodified legal texts are shipped in `assets/chamber/`; recording credits
remain in NOTICE. The preparation manifest pins every original hash, output
hash, source rate and velocity layer. Python soxr (LGPL-2.1-or-later) is an
offline authoring dependency only; no soxr code or binary is shipped in Baylee.
The runtime newly performs notes from the scores; it does not copy a finished
performance. All game compositions and their sample-bank licences are unchanged.

The [Wizards Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy)
was rechecked the same day. Existing fan-content notices remain. The new images derive from the
owner's authorized references and original image-generation calls. No Wizards
artwork/audio, third-party game art or new Scryfall request is introduced.
Photograph derivatives omit location and capture metadata. Existing fan-content
and source notices remain visible and unchanged.
