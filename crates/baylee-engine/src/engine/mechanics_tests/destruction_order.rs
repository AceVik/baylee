//! Different destruction replacement queues share one APNAP decision order.
use super::*;
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::{CounterKind, Filter, Modifier, StaticAbility, ZoneRef};

const STANDING: u32 = 7933;
const ORDINARY: u32 = 7934;
const SWEEPER: u32 = 7935;
static REGENERATE: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    source_zone: ZoneRef::Battlefield,
    layer: Modifier::RegenerateInsteadOfDestroy.layer(),
    filter: Filter::This,
    modifier: Modifier::RegenerateInsteadOfDestroy,
    condition: None,
})];
static DESTROY: &[AbilityDef] = &[free(
    &[Effect::DestroyAll {
        filter: &Filter::CREATURE,
        no_regen: false,
    }],
    None,
)];

fn setup(active: PlayerId, standing_second: bool) -> (Bench, ObjectId, ObjectId) {
    let second = if standing_second { STANDING } else { ORDINARY };
    let mut e = bench(
        7933,
        vec![
            card(
                STANDING,
                creature_face("Standing regeneration", "{0}", 2, 2),
                KeywordSet::EMPTY,
                REGENERATE,
            ),
            card(
                ORDINARY,
                creature_face("Ordinary regeneration", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                SWEEPER,
                face("Destroying artifact", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                DESTROY,
            ),
        ],
        [
            Seat::with(&[STANDING, SWEEPER]),
            Seat::with(&[second, SWEEPER]),
        ],
    );
    to_main(&mut e, active);
    let creature = |game: &Bench, player| {
        game.state()
            .battlefield_seen()
            .find(|id| {
                game.state().object(*id).is_some_and(|object| {
                    object.controller == player
                        && object.characteristics().types.contains(TypeSet::CREATURE)
                })
            })
            .unwrap()
    };
    let a = creature(&e, me());
    let b = creature(&e, them());
    e.state.object_mut(a).unwrap().regeneration_shields = 1;
    e.state.object_mut(b).unwrap().regeneration_shields = 1;
    e.state
        .object_mut(b)
        .unwrap()
        .counters
        .set(CounterKind::Shield, 1);
    (e, a, b)
}

#[test]
fn destruction_choices_merge_shield_and_regeneration_heads_in_apnap_order() {
    for active in [me(), them()] {
        for through_resolution in [false, true] {
            for standing_second in [false, true] {
                let (mut e, a, b) = setup(active, standing_second);
                if through_resolution {
                    let sweeper = objects(&e, ZoneLocation::Battlefield, SWEEPER)
                        .into_iter()
                        .find(|id| e.state().object(*id).unwrap().controller == active)
                        .unwrap();
                    activate(&mut e, active, sweeper, 0);
                    walk_until(&mut e, |game| {
                        matches!(game.pending(), Pending::YesNo { .. })
                    });
                } else {
                    crate::sba::destroy_all(&mut e.state, &[a, b], false);
                    e.awaiting_answer = false;
                    e.run_until_choice();
                }
                let mut asked = Vec::new();
                while let Pending::YesNo { player, prompt, .. } = e.pending().clone() {
                    asked.push(player);
                    let use_regeneration_shield = match prompt {
                        YesNoPrompt::UseShieldCounter { permanent } => {
                            assert_eq!(permanent, b);
                            false
                        }
                        YesNoPrompt::UseRegenerationShield { .. } => true,
                        other => panic!("unexpected destruction question: {other:?}"),
                    };
                    e.apply(player, PlayerAction::YesNo(use_regeneration_shield))
                        .unwrap();
                }
                let expected = match (active == me(), standing_second) {
                    (true, false) => vec![me(), them()],
                    (false, false) => vec![them(), me()],
                    (true, true) => vec![me(), them(), them()],
                    (false, true) => vec![them(), them(), me()],
                };
                assert_eq!(
                    asked, expected,
                    "APNAP spans both queues and newly exposed choices"
                );
                assert_eq!(e.state().object(a).unwrap().regeneration_shields, 0);
                assert_eq!(e.state().object(b).unwrap().regeneration_shields, 0);
                assert_eq!(
                    e.state()
                        .object(b)
                        .unwrap()
                        .counters
                        .get(CounterKind::Shield),
                    1
                );
            }
        }
    }
}
