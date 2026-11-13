//! Ivory Charm — {W} — Instant
//! Oracle: Choose one —
//! Oracle: • All creatures get -2/-0 until end of turn.
//! Oracle: • Tap target creature.
//! Oracle: • Prevent the next 1 damage that would be dealt to any target this turn.
//! Set: MIR #23 — Mirage | Scryfall ID: 0706acf6-587e-4f29-944a-fdf25aeacb6d | Oracle ID: dded2874-e80f-401a-b700-52c54dd6da60
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IVORY_CHARM,
    oracle_id = "dded2874-e80f-401a-b700-52c54dd6da60",
    scryfall_id = "0706acf6-587e-4f29-944a-fdf25aeacb6d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ivory Charm",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(&[Effect::PumpFilter {
                filter: &Filter::CREATURE,
                controlled_by: None,
                power: Amount::NegXFixed(2),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]),
            mode!(
                &[Effect::TapTarget],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
            mode!(
                &[Effect::PreventNextDamage {
                    target: TargetSpec::AnyTarget,
                    amount: Amount::Fixed(1),
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
            ),
        ],
    }],
);
