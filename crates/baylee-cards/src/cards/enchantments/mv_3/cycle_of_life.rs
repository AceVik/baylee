//! Cycle of Life — {1}{G}{G} — Enchantment
//! Oracle: Return this enchantment to its owner's hand: Target creature you cast this turn has base power and toughness 0/1 until your next upkeep. At the beginning of your next upkeep, put a +1/+1 counter on that creature.
//! Set: MIR #211 — Mirage | Scryfall ID: 49b53861-97aa-4fff-9526-7d496d1717a4 | Oracle ID: 5941374b-eeb8-4d0e-9367-f7f0f8b79739
// NOT SUPPORTED: Filtering for a creature you cast this turn (rather than `Filter::EnteredThisTurn`) and scheduling a delayed trigger at the beginning of your next upkeep that puts a +1/+1 counter on that targeted creature are not yet supported.

use baylee_cards_dsl::prelude::*;

const YOUR_CREATURE_ENTERED_THIS_TURN: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::EnteredThisTurn]);

card!(
    index = index::CYCLE_OF_LIFE,
    oracle_id = "5941374b-eeb8-4d0e-9367-f7f0f8b79739",
    scryfall_id = "49b53861-97aa-4fff-9526-7d496d1717a4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Filtering for a creature you cast this turn (rather than `Filter::EnteredThisTurn`) and the delayed trigger putting a +1/+1 counter on that creature at the beginning of your next upkeep are not yet supported; in play, returning Cycle of Life to its owner's hand sets target creature you control that entered the battlefield this turn to base power and toughness 0/1 until your next upkeep.",
    ),
    faces = &[face!(
        name = "Cycle of Life",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(ReturnSelfToHand),
        &[Effect::continuous(
            &Filter::This,
            Modifier::SetPT(0, 1),
            Duration::UntilYourNextUpkeep,
        )],
        target = Some(TargetSpec::Object(&YOUR_CREATURE_ENTERED_THIS_TURN)),
    )],
);
