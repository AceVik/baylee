//! Drafna's Restoration — {U} — Sorcery
//! Oracle: Put any number of target artifact cards from target player's graveyard on top of their library in any order.
//! Set: ATQ #8 — Antiquities | Scryfall ID: 4be2aa3b-207b-4d21-abfb-6788520c7676 | Oracle ID: 24a78116-d5e0-4e30-b192-d8bffe347a0c
// PARTIAL — the whole sentence is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAFNA_S_RESTORATION,
    oracle_id = "24a78116-d5e0-4e30-b192-d8bffe347a0c",
    scryfall_id = "4be2aa3b-207b-4d21-abfb-6788520c7676",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Drafna's Restoration",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no effect moves any number of cards from a graveyard to the top of \
         a library in a chosen order, and there is no target spec for cards \
         in a target player's graveyard: Effect::GraveyardToTop takes exactly \
         one target card and TargetSpec::CardInGraveyard takes a PlayerRel, \
         whose Chosen is a resolution-time choice rather than the printed \
         target"
    ),
    // NOT SUPPORTED: "Put any number of target artifact cards from target
    // player's graveyard on top of their library in any order." — the closest
    // piece, `Effect::GraveyardToTop`, moves exactly one target card, so it
    // cannot say "any number"; no effect orders several cards onto a library;
    // and `TargetSpec::CardInGraveyard` names the graveyard's player with a
    // `PlayerRel`, none of which is the targeted player the card prints
    // (`Chosen` asks which graveyard as the spell resolves). So the whole
    // sentence comes off the card.
    abilities = &[],
);
