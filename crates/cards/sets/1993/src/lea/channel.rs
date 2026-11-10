//! Channel — {G}{G} — Sorcery
//! Oracle: Until end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}.
//! Set: IMA #157 — Iconic Masters | Scryfall ID: ce54c7c1-3401-4414-8da0-5846cb0ae1b4 | Oracle ID: d1b815d1-2848-40d4-a555-66822d1becbc
// IMPLEMENTED — a special action granted until end of turn: pay 1 life, add {C}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHANNEL,
    oracle_id = "d1b815d1-2848-40d4-a555-66822d1becbc",
    scryfall_id = "ce54c7c1-3401-4414-8da0-5846cb0ae1b4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Channel",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::GrantSpecialActionUntilEndOfTurn {
        timing: SpecialActionTiming::ManaAbility,
        cost: SpecialActionCost::Life(1),
        effect: SpecialActionEffect::AddMana {
            color: ManaColor::Colorless,
            amount: 1
        },
    }])],
);
