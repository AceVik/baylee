//! `cards/creatures/mv_5/aven_flock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aven Flock prints two lines and the scenario plays both, because either one
/// alone leaves the other unread: a {4}{W} 2/3 Bird Soldier that arrives with
/// `Flying`, and "{W}: This creature gets +0/+1 until end of turn". Six Plains
/// pay the cast out of five of them and keep the sixth back for the pump, so
/// the offer is read where `can_afford` reads it — the pool — before and after
/// the last land is tapped: an empty pool leaves the ability off the offer, and
/// one white on it pays the price. The Troll beside the Flock and the turn
/// walked afterwards are the controls: `Filter::This` is the Flock and no other
/// creature, and `Duration::UntilEndOfTurn` is why the fourth point of
/// toughness is gone by the next main phase rather than a second body.
#[test]
fn aven_flock_flies_and_pumps_its_own_toughness_for_white_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                young_wolf(),
            ],
        )
        .hand(0, &[aven_flock()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is out");
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "a printed 1/1 before anything happens"
    );

    // Five of the six Plains pay the {4}{W} and the sixth is kept back for the
    // activated ability. The Wolf has no mana ability of its own, so the pool
    // is exactly the five white that were tapped for — and no more.
    let kept = on_battlefield(&engine, p0, plains()).expect("a Plains to keep back");
    let taken = tap_mana_except(&mut engine, p0, kept);
    assert_eq!(
        taken, 5,
        "the other five Plains, and nothing else on this board taps for mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five white, with one Plains still standing to pay the pump"
    );
    cast_with_floating(&mut engine, p0, aven_flock());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, aven_flock()).is_some()
    });

    let flock = on_battlefield(&engine, p0, aven_flock()).expect("the Flock resolved");
    assert_eq!(pt(&engine, flock), (2, 3), "the printed 2/3 body");
    assert!(
        keywords(&engine, flock).contains(KeywordSet::FLYING),
        "which the printed `Flying` reaches through the layers"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{W}} took every white that was floating"
    );

    // The pump's price is {W}, and `legal.abilities` is filtered through
    // `can_afford`, which reads the pool rather than the untapped lands: with
    // nothing floating the line is absent from the offer altogether.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(flock, 0)),
        "an empty pool pays no {{W}}, so the pump is not offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the one Plains kept back from the cast is the whole pool"
    );
    activate(&mut engine, p0, aven_flock(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, flock),
        (2, 4),
        "+0/+1 on the toughness and nothing at all on the power"
    );
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "`Filter::This` is the Flock: the other creature you control keeps its body"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} came out of the pool"
    );

    // "until end of turn": a turn later the Flock is the printed 2/3 again, so
    // the fourth point of toughness was a duration and not a second body.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, flock),
        (2, 3),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, aven_flock()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
