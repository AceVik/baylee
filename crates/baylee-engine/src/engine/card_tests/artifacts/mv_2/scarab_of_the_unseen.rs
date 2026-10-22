//! `cards/artifacts/mv_2/scarab_of_the_unseen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, HOLY_STRENGTH};

fn scarab_of_the_unseen() -> CardIndex {
    card_index("5f085af7-ce66-4019-9516-d1650ee7b6e0")
}

/// Scarab of the Unseen ({2} Artifact, Partial):
/// "{T}, Sacrifice this artifact: Return all Auras attached to target permanent you own to their owners' hands. Draw a card at the beginning of the next turn's upkeep."
#[test]
fn scarab_of_the_unseen_returns_auras_on_owned_permanent_and_draws_next_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[scarab_of_the_unseen(), plains(), GRIZZLY_BEARS])
        .hand(0, &[HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scarab = on_battlefield(&engine, p0, scarab_of_the_unseen()).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    // Enchant Grizzly Bears with Holy Strength.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, HOLY_STRENGTH);
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
    assert_eq!(pt(&engine, bears), (3, 4));

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
    assert!(in_hand(&engine, p0, HOLY_STRENGTH).is_some());
    assert_eq!(pt(&engine, bears), (2, 2));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
