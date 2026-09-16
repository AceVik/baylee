//! Faithless Looting — {R} — Sorcery
//! Oracle: Draw two cards, then discard two cards.
//! Oracle: Flashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: SOC #244 — Secrets of Strixhaven Commander | Scryfall ID: fc019ffa-4461-4f3d-ab8d-e4d20e77ca0c | Oracle ID: 3d6fa57a-aa53-4b5c-b8af-a7612c823117
// PARTIAL — a one-mana red loot at sorcery speed: draw two cards, then
// discard two.
// NOT SUPPORTED: `Flashback {2}{R}`. No `FaceDef` field and no `AbilityDef`
// says "you may cast this card from your graveyard for {2}{R}, then exile
// it". `FaceDef::disturb` is the one graveyard cast a face can print and it
// pays that face's own *mana cost*, which is {R} here and not the printed
// flashback cost; `Modifier::GrantsFlashback` grants the permission to a
// *target* card and also at that card's mana cost, and a static ability on a
// sorcery is never in effect to point at itself (CR 113.6). Flashback is not
// on `keyword_tests::ENFORCED`, so it is no `KeywordSet` bit either. Without
// the line the card plays exactly as though it were not printed: a plain {R}
// sorcery that loots two, once.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAITHLESS_LOOTING,
    oracle_id = "3d6fa57a-aa53-4b5c-b8af-a7612c823117",
    scryfall_id = "fc019ffa-4461-4f3d-ab8d-e4d20e77ca0c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Faithless Looting",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial("flashback {2}{R} is not written: it casts only from hand"),
    abilities = &[spell!(&[
        Effect::draw(2),
        Effect::DiscardForPlayers {
            who: PlayerRel::You,
            count: 2,
        },
    ])],
);

// Behaviour belongs in `baylee-engine`'s `engine::card_tests`: this is the
// same draw-then-discard list Frantic Search writes, but it *ends* on the
// discard, so resolution suspends on that choice with nothing left to resume
// into. Whichever of the two lands first is the one that plays it.
