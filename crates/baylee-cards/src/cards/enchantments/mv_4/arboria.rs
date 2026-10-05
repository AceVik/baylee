//! Arboria — {2}{G}{G} — World Enchantment
//! Oracle: Creatures can't attack a player unless that player cast a spell or put a nontoken permanent onto the battlefield during their last turn.
//! Set: DMR #149 — Dominaria Remastered | Scryfall ID: b18f4de2-adfe-4784-bd57-9e2ad9088825 | Oracle ID: acb3e93c-a1d3-458f-b8c3-c426cd359fa4
// PARTIAL — no condition or modifier reads what a player did during their
// last turn, so the attack restriction is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARBORIA,
    oracle_id = "acb3e93c-a1d3-458f-b8c3-c426cd359fa4",
    scryfall_id = "b18f4de2-adfe-4784-bd57-9e2ad9088825",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Condition or Modifier reads what a player did during their last \
         turn — a spell cast or a nontoken permanent put onto the battlefield \
         — so the per-defender attack restriction is not expressible"
    ),
    faces = &[face!(
        name = "Arboria",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    // NOT SUPPORTED: "Creatures can't attack a player unless that player cast
    // a spell or put a nontoken permanent onto the battlefield during their
    // last turn." — `Modifier::CantAttackUnlessDefenderControls` reads a
    // permanent the defender controls now and not their history;
    // `Modifier::CantBeAttackedExceptBy` filters the attacking creatures;
    // `Condition::APlayerCastLastTurnAtLeast` and
    // `Condition::NoSpellsCastLastTurn` are game-wide spell facts with no
    // per-player reading and no "or put a nontoken permanent onto the
    // battlefield" half. An ungated attack ban would be a different card.
    abilities = &[],
);
