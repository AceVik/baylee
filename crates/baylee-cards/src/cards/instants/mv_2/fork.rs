//! Fork — {R}{R} — Instant
//! Oracle: Copy target instant or sorcery spell, except that the copy is red. You may choose new targets for the copy.
//! Set: ME4 #116 — Masters Edition IV | Scryfall ID: e4ff994a-bddd-486d-9a7b-a8959b4cf1dd | Oracle ID: 50c53ae0-51ba-4046-ac74-87c65e688032
// PARTIAL — a copy of a spell aimed at a player keeps that player: only object
// targets may be chosen anew.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORK,
    oracle_id = "50c53ae0-51ba-4046-ac74-87c65e688032",
    scryfall_id = "e4ff994a-bddd-486d-9a7b-a8959b4cf1dd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "a copy of a spell aimed at a player keeps that player: only object targets may be chosen anew"
    ),
    faces = &[face!(
        name = "Fork",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::CopyTargetSpell {
                mods: &[CopyMod::SetColor(ColorSet::from_slice(&[Color::Red]))]
            }],
            targets = Some(TargetReq::one(TargetSpec::Spell(
                &Filter::INSTANT_OR_SORCERY
            )))
        ),
        // NOT SUPPORTED: You may choose new targets for the copy. (A player target is
        // kept.)
    ],
);
