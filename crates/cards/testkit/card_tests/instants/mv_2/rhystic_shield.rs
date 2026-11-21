//! `cards/instants/mv_2/rhystic_shield.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "a905dafa-1818-471a-a485-d84be447263b"
fn rhystic_shield() -> CardIndex {
    card_index("a905dafa-1818-471a-a485-d84be447263b")
}

/// Rhystic Shield ({1}{W}): when nobody pays {2}, creatures you control get
/// +0/+3 total (+0/+1 plus +0/+2) until end of turn; opponent's creatures are
/// untouched, and the bonus wears off next turn.
#[test]
fn rhystic_shield_gives_plus_zero_plus_three_when_unpaid() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[index::GRIZZLY_BEARS],
        &[rhystic_shield()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let my_lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, rhystic_shield());
    // APNAP order: P0 declines, then P1 declines.
    for expected in [P0, P1] {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        assert!(matches!(
            engine.pending(),
            Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
                if *player == expected && *cost == baylee_core::mana!("{2}")
        ));
        engine.apply(expected, PlayerAction::YesNo(false)).unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_bears), (2, 5), "+0/+3 when unpaid");
    assert_eq!(pt(&engine, my_lions), (2, 4), "+0/+3 on all your creatures");
    assert_eq!(
        pt(&engine, their_bears),
        (2, 2),
        "opponent's creature untouched"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_bears), (2, 2), "bonus wears off at EOT");
    assert_eq!(pt(&engine, my_lions), (2, 1), "bonus wears off at EOT");
}

/// When an opponent pays {2}, Rhystic Shield grants only the base +0/+1 until
/// end of turn and spends the opponent's {2}.
#[test]
fn rhystic_shield_gives_only_plus_zero_plus_one_when_opponent_pays_two() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[rhystic_shield()], &[]);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    lk::float(&mut engine, P1, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, rhystic_shield());
    // APNAP order: P0 declines, then P1 pays {2}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
            if *player == P0 && *cost == baylee_core::mana!("{2}")
    ));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        panic!("expected PayMana prompt for P1, got {:?}", engine.pending());
    };
    assert_eq!(player, P1, "P1 is offered to pay {{2}}");
    assert_eq!(cost, baylee_core::mana!("{2}"));
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P1), 0, "P1 spent {{2}}");
    assert_eq!(
        pt(&engine, my_bears),
        (2, 3),
        "only +0/+1 when {{2}} is paid"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_bears), (2, 2), "base +0/+1 wears off at EOT");
}
