//! Leviathan — {5}{U}{U}{U}{U} — Creature — Leviathan
//! Oracle: Trample
//! Oracle: This creature enters tapped and doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may sacrifice two Islands. If you do, untap this creature.
//! Oracle: This creature can't attack unless you sacrifice two Islands. (This cost is paid as attackers are declared.)
//! Set: TSB #23 — Time Spiral Timeshifted | Scryfall ID: bc482c8f-5645-4cf9-8e90-090c77f9dc64 | Oracle ID: d8c49932-9b4f-4cb0-a7e9-4b7eb12203ee

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
static YOUR_ISLAND: Filter = Filter::And(&[ISLAND, Filter::ControlledByYou]);

card!(
    index = index::LEVIATHAN,
    oracle_id = "d8c49932-9b4f-4cb0-a7e9-4b7eb12203ee",
    scryfall_id = "bc482c8f-5645-4cf9-8e90-090c77f9dc64",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Leviathan",
        mana_cost = mana!("{5}{U}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LEVIATHAN],
        power = Some(10),
        toughness = Some(10),
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMaySacrificeNThen {
                player: PlayerRel::You,
                filter: &YOUR_ISLAND,
                count: 2,
                effect: &Effect::UntapSelf,
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessYouSacrifice {
                count: 2,
                filter: &ISLAND,
            }
        ),
    ],
);
