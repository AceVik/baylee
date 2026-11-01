//! `cards/artifacts/mv_2/scarab_of_the_unseen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, HOLY_STRENGTH};

fn scarab_of_the_unseen() -> CardIndex {
    card_index("5f085af7-ce66-4019-9516-d1650ee7b6e0")
}

/// Cast Holy Strength on `creature` from the mana floating.
fn enchant(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    let p0 = PlayerId::new(0);
    cast_with_floating(engine, p0, HOLY_STRENGTH);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![creature],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Scarab of the Unseen ({2} Artifact):
/// "{T}, Sacrifice this artifact: Return all Auras attached to target permanent you own to their owners' hands. Draw a card at the beginning of the next turn's upkeep."
#[test]
fn scarab_of_the_unseen_returns_only_the_auras_on_the_target_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                scarab_of_the_unseen(),
                plains(),
                plains(),
                GRIZZLY_BEARS,
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[swamp()])
        .hand(0, &[HOLY_STRENGTH, HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scarab = on_battlefield(&engine, p0, scarab_of_the_unseen()).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let their_swamp = on_battlefield(&engine, p1, swamp()).unwrap();

    // An Aura on each of two creatures you own.
    tap_all_mana(&mut engine, p0);
    enchant(&mut engine, bears);
    enchant(&mut engine, giant);
    assert_eq!(pt(&engine, bears), (3, 4));
    assert_eq!(pt(&engine, giant), (4, 5));

    // Activate Scarab of the Unseen targeting Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scarab,
                ability_index: 0,
            },
        )
        .unwrap();
    // "Target permanent you own": the opponent's Swamp is not offered.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![their_swamp],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, scarab_of_the_unseen()).is_some());
    // The Aura on the target went home; the one on the other creature stayed.
    assert!(in_hand(&engine, p0, HOLY_STRENGTH).is_some());
    assert_eq!(pt(&engine, bears), (2, 2));
    assert!(on_battlefield(&engine, p0, HOLY_STRENGTH).is_some());
    assert_eq!(pt(&engine, giant), (4, 5), "the other Aura stays on");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
