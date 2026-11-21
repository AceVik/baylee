//! `cards/instants/mv_2/wild_might.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3f8fa982-73ee-410a-8873-0afc8a6f11fc"
fn wild_might() -> CardIndex {
    card_index("3f8fa982-73ee-410a-8873-0afc8a6f11fc")
}

/// Wild Might ({1}{G}): when nobody pays {2}, target creature gets +5/+5
/// (+1/+1 plus +4/+4) until end of turn, then returns to normal next turn.
#[test]
fn wild_might_gives_plus_five_plus_five_when_unpaid() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[],
        &[wild_might()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P0, wild_might());
    lk::aim(&mut engine, &[bears], &[]);
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

    assert_eq!(pt(&engine, bears), (7, 7), "+1/+1 plus +4/+4 when unpaid");
    assert_eq!(pt(&engine, lions), (2, 1), "bystander untouched");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "bonus wears off at EOT");
}

/// When an opponent pays {2}, target creature gets only the base +1/+1 until
/// end of turn and the opponent's {2} is spent.
#[test]
fn wild_might_gives_only_plus_one_plus_one_when_opponent_pays_two() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[wild_might()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    lk::float(&mut engine, P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P0, wild_might());
    lk::aim(&mut engine, &[bears], &[]);
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
    assert_eq!(player, P1);
    assert_eq!(cost, baylee_core::mana!("{2}"));
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P1), 0, "P1 spent {{2}}");
    assert_eq!(pt(&engine, bears), (3, 3), "only +1/+1 when {{2}} is paid");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "base +1/+1 wears off at EOT");
}
