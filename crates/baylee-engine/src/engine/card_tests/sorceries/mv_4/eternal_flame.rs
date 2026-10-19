//! `cards/sorceries/mv_4/eternal_flame.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn eternal_flame() -> CardIndex {
    card_index("b3119f27-45fd-4411-8a09-5f8d3cd8d927")
}

/// Seat 0 holds Eternal Flame; the lands are as given for each seat.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .hand(0, &[eternal_flame()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Eternal Flame (b3119f27-45fd-4411-8a09-5f8d3cd8d927) — "Eternal Flame
/// deals X damage to target opponent or planeswalker and half X damage,
/// rounded up, to you, where X is the number of Mountains you control." Four
/// Mountains: 4 to the opponent, 2 to me.
#[test]
fn eternal_flame_deals_x_to_the_opponent_and_half_x_to_its_caster() {
    let mut engine = board(&[mountain(), mountain(), mountain(), mountain()], &[]);
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, eternal_flame(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 16, "X = 4");
    assert_eq!(lk::life(&engine, P0), 18, "half of 4");
    assert!(in_graveyard(&engine, P0, eternal_flame()).is_some());
}

/// Eternal Flame — "rounded up", and only Mountains count, mine only: three
/// Mountains, a Forest, and two opposing Mountains give X = 3, so 3 damage
/// to them and 2 (half of 3, rounded up) to me.
#[test]
fn eternal_flame_rounds_half_up_and_counts_only_my_mountains() {
    let mut engine = board(
        &[mountain(), mountain(), mountain(), forest()],
        &[mountain(), mountain()],
    );
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, eternal_flame(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(
        lk::life(&engine, P1),
        17,
        "X = 3: the Forest and their Mountains do not count"
    );
    assert_eq!(lk::life(&engine, P0), 18, "ceil(3 / 2) = 2");
}

/// Eternal Flame — five Mountains: 5 and ceil(5 / 2) = 3.
#[test]
fn eternal_flame_with_five_mountains_hurts_me_for_three() {
    let mut engine = board(&[mountain(); 5], &[]);
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, eternal_flame(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 15);
    assert_eq!(lk::life(&engine, P0), 17);
}

/// Eternal Flame — "target opponent or planeswalker": I am not a legal
/// target, the opponent is.
#[test]
fn eternal_flame_offers_the_opponent_but_not_its_caster() {
    let mut engine = board(&[mountain(), mountain(), mountain(), mountain()], &[]);
    cast_from_hand(&mut engine, P0, eternal_flame());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&P1), "the opponent");
    assert!(!player_options.contains(&P0), "not myself");
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P0]
                }
            )
            .is_err(),
        "naming myself is refused"
    );
}
