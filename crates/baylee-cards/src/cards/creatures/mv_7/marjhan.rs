//! Marjhan — {5}{U}{U} — Creature — Serpent
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: {U}{U}, Sacrifice a creature: Untap this creature. Activate only during your upkeep.
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: {U}{U}: This creature gets -1/-0 until end of turn and deals 1 damage to target attacking creature without flying.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: ME2 #54 — Masters Edition II | Scryfall ID: 047ac2cb-3a96-4ff6-8b5a-251399974c47 | Oracle ID: 3fdee2ab-7ec6-4fc6-ad99-f04571f94583
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
const YOUR_ISLAND: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::ISLAND),
]);
const NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLAND, 0);
static ATTACKING_NONFLYER: Filter = Filter::And(&[
    Filter::ATTACKING_CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::MARJHAN,
    oracle_id = "3fdee2ab-7ec6-4fc6-ad99-f04571f94583",
    scryfall_id = "047ac2cb-3a96-4ff6-8b5a-251399974c47",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Marjhan",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(8),
        toughness = Some(8),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(
            cost!("{U}{U}", Sacrifice(&Filter::CREATURE)),
            &[Effect::UntapSelf],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ]))
        ),
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&ISLAND)
        ),
        activated!(
            cost!("{U}{U}"),
            &[
                Effect::ModifySource {
                    modifier: Modifier::ModifyPT(-1, 0),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Object(&ATTACKING_NONFLYER),
                },
            ],
            target = Some(TargetSpec::Object(&ATTACKING_NONFLYER)),
        ),
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
    ],
);
