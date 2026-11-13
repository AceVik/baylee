//! Scarwood Hag — {1}{G} — Creature — Hag
//! Oracle: {G}{G}{G}{G}, {T}: Target creature gains forestwalk until end of turn. (It can't be blocked as long as defending player controls a Forest.)
//! Oracle: {T}: Target creature loses forestwalk until end of turn.
//! Set: DRK #86 — The Dark | Scryfall ID: ac2655e4-3a4d-4f73-820a-02fab675d42e | Oracle ID: 2b483041-4dea-468a-9857-50dae2e8a57d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCARWOOD_HAG,
    oracle_id = "2b483041-4dea-468a-9857-50dae2e8a57d",
    scryfall_id = "ac2655e4-3a4d-4f73-820a-02fab675d42e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scarwood Hag",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HAG],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{G}{G}{G}{G}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::FORESTWALK),
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::RemoveKeyword(KeywordSet::FORESTWALK),
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
