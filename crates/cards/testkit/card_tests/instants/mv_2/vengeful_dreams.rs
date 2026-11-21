//! `cards/instants/mv_2/vengeful_dreams.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "55d01851-2669-4552-b580-088e445ac30c"
fn vengeful_dreams() -> CardIndex {
    card_index("55d01851-2669-4552-b580-088e445ac30c")
}

/// Vengeful Dreams ({W}{W}): discards X cards as an additional cost and exiles
/// X target attacking creatures (excluding non-attackers).
#[test]
fn vengeful_dreams_discards_x_and_exiles_x_attacking_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[vengeful_dreams(), mountain(), forest()])
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);

    // Giant and Bears attack; Lions stays home.
    let _ = lk::attack(&mut engine, &[giant, bears]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, vengeful_dreams());
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&giant) && options.contains(&bears));
                assert!(
                    !options.contains(&lions),
                    "non-attacking Lions is not a legal target: {options:?}"
                );
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![giant, bears],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, mountain()).is_some());
    assert!(in_graveyard(&engine, P0, forest()).is_some());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P1))
            .contains(&giant)
            && engine
                .state()
                .zones
                .list(ZoneLocation::Exile(P1))
                .contains(&bears),
        "both attacking creatures were exiled"
    );
    assert_eq!(
        on_battlefield(&engine, P1, index::SAVANNAH_LIONS),
        Some(lions)
    );

    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "exiled attackers dealt 0 damage");
}

/// Vengeful Dreams can be cast for X = 0 with no other cards in hand.
#[test]
fn vengeful_dreams_casts_for_x_zero_with_no_other_cards() {
    let mut engine = lk::game(SEED, &[], &[], &[vengeful_dreams()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, vengeful_dreams());
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    }
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, vengeful_dreams()).is_some());
}
