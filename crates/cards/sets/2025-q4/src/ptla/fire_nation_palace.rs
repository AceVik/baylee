//! Fire Nation Palace — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}, {T}: Target creature you control gains firebending 4 until end of turn. (Whenever it attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)
//! Set: TLA #268 — Avatar: The Last Airbender | Scryfall ID: c91ac3f2-9fcd-4b41-a168-ae7f70b67d3c | Oracle ID: f2000fb8-39c6-4ad6-a020-5245faaa1eba
// IMPLEMENTED — conditional entry, red mana, and temporary targeted
// firebending 4 whose mana lasts through the combat phase.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE_NATION_PALACE,
    oracle_id = "f2000fb8-39c6-4ad6-a020-5245faaa1eba",
    scryfall_id = "c91ac3f2-9fcd-4b41-a168-ae7f70b67d3c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fire Nation Palace",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::YOUR_BASIC_LAND)],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{1}{R}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::GrantTriggered {
                    trigger: Trigger::Attacks(&Filter::This),
                    effects: &[Effect::AddCombatMana {
                        color: ManaColor::Red,
                        amount: 4,
                    }],
                    target: None,
                },
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
    ],
);
