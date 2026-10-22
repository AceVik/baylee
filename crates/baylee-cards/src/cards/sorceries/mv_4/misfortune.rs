//! Misfortune — {1}{B}{R}{G} — Sorcery
//! Oracle: An opponent chooses one —
//! Oracle: • You put a +1/+1 counter on each creature you control and gain 4 life.
//! Oracle: • You put a -1/-1 counter on each creature that player controls and Misfortune deals 4 damage to that player.
//! Set: ALL #113 — Alliances | Scryfall ID: b14cc32a-eb4f-4690-aceb-160780743ebe | Oracle ID: e282eb5b-f9e6-4fb3-9edf-44e23359357d
// NOT SUPPORTED: having an opponent choose one of the two modes requires opponent-chosen modal spells.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISFORTUNE,
    oracle_id = "e282eb5b-f9e6-4fb3-9edf-44e23359357d",
    scryfall_id = "b14cc32a-eb4f-4690-aceb-160780743ebe",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    coverage = Coverage::Partial(
        "Having an opponent choose one of the two modes requires opponent-chosen modal spells; in play, Misfortune's caster chooses between putting a +1/+1 counter on each creature they control and gaining 4 life, or putting a -1/-1 counter on each creature opponents control and dealing 4 damage to each opponent.",
    ),
    faces = &[face!(
        name = "Misfortune",
        mana_cost = mana!("{1}{B}{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(&[
                Effect::AddCounterFilter {
                    filter: &Filter::YOUR_CREATURE,
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::GainLife {
                    amount: Amount::Fixed(4),
                },
            ]),
            mode!(&[
                Effect::AddCounterFilter {
                    filter: &Filter::OPPONENT_CREATURE,
                    kind: CounterKind::M1M1,
                    amount: Amount::Fixed(1),
                },
                Effect::DealDamage {
                    amount: Amount::Fixed(4),
                    target: TargetSpec::Player(PlayerRel::EachOpponent),
                },
            ]),
        ],
    }],
);
