//! Open Communications — {U} — Sorcery
//! Oracle: Draw a card.
//! Oracle: Beam me up {2}{U} (You may cast this card from your graveyard for {2}{U} if you also return a creature you control to its owner's hand. Then exile this spell.)
//! Set: TRK #76 — Star Trek | Scryfall ID: 8eeec269-13f0-4923-9f13-e995acd73e00 | Oracle ID: b08c5b86-4f84-46a3-877f-bbf71ec17adb
// PARTIAL — a one-mana blue cantrip at sorcery speed: draw a card.
// NOT SUPPORTED: `Beam me up {2}{U}`. Nothing in the DSL says "you may cast
// this card from your graveyard for {2}{U} if you also return a creature you
// control to its owner's hand, then exile it", and it fails on all three
// halves. `FaceDef::disturb` is the one graveyard cast a face can print and
// it pays that face's own *mana cost*, which is {U} here and not the printed
// {2}{U}. `AlternativeCost` carries no zone and `AltCondition` is
// `Always`/`NotYourTurn`/`CommanderControlled`, so no alternative cost can be
// restricted to a graveyard. And no `CostPart` returns another permanent to
// its owner's hand — `ReturnSelfToHand` returns the source, which a sorcery
// on the stack is not. "Beam me up" is not on `keyword_tests::ENFORCED`, so
// it is no `KeywordSet` bit either. Without the line the card plays exactly
// as though it were not printed: a {U} sorcery that cantrips once, from hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OPEN_COMMUNICATIONS,
    oracle_id = "b08c5b86-4f84-46a3-877f-bbf71ec17adb",
    scryfall_id = "8eeec269-13f0-4923-9f13-e995acd73e00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Open Communications",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial("beam me up {2}{U} is not written: it casts only from hand"),
    abilities = &[spell!(&[Effect::draw(1)])],
);

// Behaviour belongs in `baylee-engine`'s `engine::card_tests`: the card is one
// `Effect::draw(1)` on resolution, which the s4 scenario tests already cover,
// and with the graveyard cast unwritten there is nothing else here to play.
