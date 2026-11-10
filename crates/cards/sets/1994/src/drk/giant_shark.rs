//! Giant Shark — {5}{U} — Creature — Shark
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn, this creature gets +2/+0 and gains trample until end of turn.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: DRK #29 — The Dark | Scryfall ID: 53ec4a19-0f2f-4713-a869-58832484648d | Oracle ID: 44a10a63-be9c-4f1d-aad6-b5337112bda5

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
static YOUR_ISLAND: Filter = Filter::And(&[ISLAND, Filter::ControlledByYou]);
static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLAND, 0);
static DAMAGED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::DealtDamageThisTurn]);

card!(
    index = index::GIANT_SHARK,
    oracle_id = "44a10a63-be9c-4f1d-aad6-b5337112bda5",
    scryfall_id = "53ec4a19-0f2f-4713-a869-58832484648d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Giant Shark",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHARK],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&ISLAND)
        ),
        triggered!(
            Trigger::BlocksOrBecomesBlockedBy(&DAMAGED_CREATURE),
            &[
                Effect::ModifySource {
                    modifier: Modifier::ModifyPT(2, 0),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    duration: Duration::UntilEndOfTurn,
                },
            ]
        ),
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
    ],
);
