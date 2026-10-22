//! `cards/instants/mv_3/wind_shear.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, SERRA_ANGEL};

fn wind_shear() -> CardIndex {
    card_index("fa683059-b9de-4063-ae71-597ed4e2df30")
}

/// `Wind Shear` ({2}{G}, Instant, `Coverage::Implemented`) gives attacking
/// creatures with flying -2/-2 and removes flying from them until end of turn.
#[test]
fn wind_shear_shrinks_attacking_flyers_and_removes_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[SERRA_ANGEL, GRIZZLY_BEARS])
        .hand(0, &[wind_shear()])
        .start();
    keep_mulligans(&mut engine);

    let angel = on_battlefield(&engine, p1, SERRA_ANGEL).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p0)), (bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    cast_from_hand(&mut engine, p0, wind_shear());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });

    assert_eq!(pt(&engine, angel), (2, 2));
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, bear), (2, 2));
}
