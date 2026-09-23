//! Sevinne's Reclamation — {2}{W} — Sorcery
//! Oracle: Return target permanent card with mana value 3 or less from your graveyard to the battlefield. If this spell was cast from a graveyard, you may copy this spell and may choose a new target for the copy.
//! Oracle: Flashback {4}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: SOC #170 — Secrets of Strixhaven Commander | Scryfall ID: 8deab1ef-4219-4767-a3c4-b61250d0ebe0 | Oracle ID: 1b9f9f5b-8712-4f00-90cb-1b7b9970eccc
// PARTIAL — a white sorcery that returns one permanent card of mana value 3
// or less from your own graveyard to the battlefield; cast from hand, that is
// the whole card.
//
// NOT SUPPORTED: `Flashback {4}{W}`, exactly as on Faithless Looting. No
// `FaceDef` field and no `AbilityDef` says "you may cast this card from your
// graveyard for {4}{W}, then exile it". `FaceDef::disturb` is the one
// graveyard cast a face can print and it pays that face's own *mana cost*,
// which is {2}{W} here and not the printed flashback cost;
// `Modifier::GrantsFlashback` grants the permission to a *target* card and
// also at that card's mana cost, and a static ability on a sorcery is never
// in effect to point at itself (CR 113.6). Flashback is not on
// `keyword_tests::ENFORCED`, so it is no `KeywordSet` bit either.
//
// NOT SUPPORTED: `If this spell was cast from a graveyard, you may copy this
// spell and may choose a new target for the copy.` Two halves and neither is
// sayable. No `Effect` copies the spell it is itself part of —
// `Effect::CopyTargetSpell` copies the *first target*, and this spell's one
// target is a card in a graveyard rather than a spell on the stack. And no
// condition names the zone a spell was cast from: the `Effect::If…` family
// reads kicker, creatures that died, life lost, the greatest mana value and
// an event's power, while the engine's own `Rider::Flashback` and
// `GameObject::cast_from_hand` sit behind no DSL variant at all. With the
// flashback line unwritten this spell casts only from hand, so the condition
// is false everywhere the card can be played by itself and the rider changes
// nothing; behind a *granted* flashback (Snapcaster Mage, Emry) it is a copy
// that is silently not offered.

use baylee_cards_dsl::prelude::*;

/// "Permanent card with mana value 3 or less" — the same three clauses Sun
/// Titan writes for the same printed sentence, so the two cards are provably
/// reading one filter and not two: in this pool a permanent card is a card
/// that is neither an instant nor a sorcery.
static SMALL_PERMANENT: Filter = Filter::And(&[
    Filter::CmcAtMost(3),
    Filter::LacksType(TypeSet::INSTANT),
    Filter::LacksType(TypeSet::SORCERY),
]);

card!(
    index = index::SEVINNE_S_RECLAMATION,
    oracle_id = "1b9f9f5b-8712-4f00-90cb-1b7b9970eccc",
    scryfall_id = "8deab1ef-4219-4767-a3c4-b61250d0ebe0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sevinne's Reclamation",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial("flashback {4}{W} and the copy it enables are not written"),
    abilities = &[spell!(
        &[Effect::reanimate(TargetSpec::CardInGraveyard(
            &SMALL_PERMANENT,
            PlayerRel::You
        ))],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &SMALL_PERMANENT,
            PlayerRel::You,
        )))
    )],
);

// No new mechanic to play: Reanimate already casts a spell whose
// `TargetReq::one` over `TargetSpec::CardInGraveyard` feeds
// `Effect::GraveyardToBattlefield`, and Sun Titan already points that effect
// at `SMALL_PERMANENT`. This card is the two of them joined, so whatever
// `engine::card_tests` proves about either proves this one too.
