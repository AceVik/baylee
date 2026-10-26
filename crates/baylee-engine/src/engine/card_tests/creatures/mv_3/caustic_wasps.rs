//! `cards/creatures/mv_3/caustic_wasps.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "fea997a4-3ed1-4f83-b16a-56939f4b2211"
fn caustic_wasps() -> CardIndex {
    card_index("fea997a4-3ed1-4f83-b16a-56939f4b2211")
}

/// Caustic Wasps ({2}{G} 1/1 Flying Creature — Insect):
/// "Whenever this creature deals combat damage to a player, you may destroy
/// target artifact that player controls."
#[test]
fn caustic_wasps_destroys_target_artifact_damaged_player_controls() {
    let mut engine = lk::game(
        SEED,
        &[caustic_wasps(), index::JAYEMDAE_TOME],
        &[index::ROD_OF_RUIN, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let wasps = lk::obj(&engine, P0, caustic_wasps());
    let my_tome = lk::obj(&engine, P0, index::JAYEMDAE_TOME);
    let their_rod = lk::obj(&engine, P1, index::ROD_OF_RUIN);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let offer = lk::attack(&mut engine, &[wasps]);
    assert!(
        !lk::may_block(&offer, their_bears, wasps),
        "ground Grizzly Bears cannot block flying Caustic Wasps"
    );
    lk::block(&mut engine, &[]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&their_rod),
        "artifact controlled by damaged player is a legal target"
    );
    assert!(
        !options.contains(&my_tome),
        "artifact controlled by controller is not a legal target"
    );
    lk::aim(&mut engine, &[their_rod], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::through_combat(&mut engine);

    assert_eq!(lk::life(&engine, P1), 19);
    assert!(
        in_graveyard(&engine, P1, index::ROD_OF_RUIN).is_some(),
        "Caustic Wasps destroyed target artifact controlled by the damaged player"
    );
    assert!(
        on_battlefield(&engine, P0, index::JAYEMDAE_TOME).is_some(),
        "controller's artifact remains on the battlefield"
    );
}
