//! Gaea's Liege — {3}{G}{G}{G} — Creature — Avatar
//! Oracle: As long as Gaea's Liege isn't attacking, its power and toughness are each equal to the number of Forests you control. As long as Gaea's Liege is attacking, its power and toughness are each equal to the number of Forests defending player controls.
//! Oracle: {T}: Target land becomes a Forest until this creature leaves the battlefield.
//! Set: TSB #78 — Time Spiral Timeshifted | Scryfall ID: 3ade8d4a-6a47-4a01-9a0f-ff866055fd49 | Oracle ID: 8d134a60-e1e5-4163-8bdc-36af91567185

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// Each sentence sets power and toughness only while a condition holds, so
// neither is a characteristic-defining ability (CR 604.3a, its fifth
// criterion): both are layer 7b effects of statics that exist while their
// condition does, and off the battlefield the card is its printed */*, 0/0.
// While it is being declared as an attacker it is not attacking yet, so the
// first sentence applies until the declaration is complete (the card's
// ruling of 2004-10-04).
static FORESTS: Filter = Filter::HasSubtype(subtypes::land::FOREST);
static NOT_ATTACKING: Filter = Filter::Not(&Filter::Attacking);

card!(
    index = index::GAEA_S_LIEGE,
    oracle_id = "8d134a60-e1e5-4163-8bdc-36af91567185",
    scryfall_id = "3ade8d4a-6a47-4a01-9a0f-ff866055fd49",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gaea's Liege",
        mana_cost = mana!("{3}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::SetPTToCount(PtCount::YouControl(&FORESTS)),
            condition = Some(Condition::SourceMatches(&NOT_ATTACKING))
        ),
        activated!(
            Cost::TAP,
            &[Effect::continuous(
                &Filter::This,
                Modifier::SetLandType(subtypes::land::FOREST),
                Duration::WhileSourceOnBattlefield
            )],
            target = Some(TargetSpec::Object(&Filter::LAND))
        ),
        // The first sentence's second half, after the {T} ability so that
        // ability keeps index 1.
        static_ability!(
            Filter::This,
            Modifier::SetPTToCount(PtCount::DefendingPlayerControls(&FORESTS)),
            condition = Some(Condition::SourceMatches(&Filter::Attacking))
        ),
    ],
);
