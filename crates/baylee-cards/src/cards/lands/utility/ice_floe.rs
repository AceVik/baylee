//! Ice Floe — (no cost) — Land
//! Oracle: You may choose not to untap this land during your untap step.
//! Oracle: {T}: Tap target creature without flying that's attacking you. It doesn't untap during its controller's untap step for as long as this land remains tapped.
//! Set: ME2 #232 — Masters Edition II | Scryfall ID: 9a974983-b9aa-4f12-8279-2e74089f7f31 | Oracle ID: cfaaead2-09e8-47cb-9e39-8570b8d8de86
// IMPLEMENTED — the untap-step choice, the tap, and the lock that keeps the
// creature down for as long as this land remains tapped
// (Modifier::DoesNotUntap, Duration::WhileSourceTapped).

use baylee_cards_dsl::prelude::*;

// "attacking you" is read the way `PlayerRel::Opponent` is — heads-up, an
// attacker an opponent controls.
static ATTACKING_NONFLYER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
    Filter::Attacking,
    Filter::ControlledByOpponent,
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
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::WhileSourceTapped
                ),
            ],
            target = Some(TargetSpec::Object(&ATTACKING_NONFLYER))
        ),
    ],
);
