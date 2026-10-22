//! `cards/creatures/mv_6/firestorm_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn firestorm_hellkite() -> CardIndex {
    card_index("533e41cc-7cae-47f2-886b-8a3ffb82bf81")
}

/// `Firestorm Hellkite` ({4}{U}{R}, 6/6 Dragon, `Coverage::Implemented`) has
/// flying, trample, and cumulative upkeep {U}{R}.
#[test]
fn firestorm_hellkite_has_flying_trample_and_cumulative_upkeep_ur() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[firestorm_hellkite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, firestorm_hellkite());
    pass_until(&mut engine, stack_is_empty);
    let hellkite = on_battlefield(&engine, p0, firestorm_hellkite()).unwrap();
    assert_eq!(pt(&engine, hellkite), (6, 6));
    assert!(keywords(&engine, hellkite).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, hellkite).contains(KeywordSet::TRAMPLE));

    // Advance to P0's next upkeep and decline the cumulative upkeep payment.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(hellkite).unwrap().zone,
        Zone::Graveyard
    );
}
