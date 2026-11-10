//! Ice Floe — (no cost) — Land
//! Oracle: You may choose not to untap this land during your untap step.
//! Oracle: {T}: Tap target creature without flying that's attacking you. It doesn't untap during its controller's untap step for as long as this land remains tapped.
//! Set: ME2 #232 — Masters Edition II | Scryfall ID: 9a974983-b9aa-4f12-8279-2e74089f7f31 | Oracle ID: cfaaead2-09e8-47cb-9e39-8570b8d8de86

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static ATTACKING_NONFLYER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
    Filter::AttackingYou,
]);

card!(
    index = index::ICE_FLOE,
    oracle_id = "cfaaead2-09e8-47cb-9e39-8570b8d8de86",
    scryfall_id = "9a974983-b9aa-4f12-8279-2e74089f7f31",
    faces = &[face!(name = "Ice Floe", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[
                Effect::TapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false
                    }
                }
            ],
            target = Some(TargetSpec::Object(&ATTACKING_NONFLYER))
        ),
    ],
);
