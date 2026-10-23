//! `cards/enchantments/mv_1/eladamri_s_vineyard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn eladamri_s_vineyard() -> CardIndex {
    card_index("faa085c3-705f-465d-9290-0e22276ad06d")
}

/// Eladamri's Vineyard (`faa085c3-705f-465d-9290-0e22276ad06d`) — {G} Enchantment:
/// "At the beginning of each player's first main phase, that player adds {G}{G}."
#[test]
fn eladamri_s_vineyard_adds_two_green_at_each_players_first_main_phase() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eladamri_s_vineyard()])
        .start();
    keep_mulligans(&mut engine);

    // P0's first main phase begins and triggers Eladamri's Vineyard.
    reach_main_phase(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    // Advance to P1's first main phase; P1 gets {G}{G}.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        2
    );
}

#[test]
fn eladamris_vineyard_does_not_add_mana_in_second_main() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(SEED, &[eladamri_s_vineyard()], &[], &[], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::pool_of(&e, lk::P0, ManaColor::Green), 2);
    assert_eq!(lk::pool_total(&e, lk::P1), 0);
    pass_until(&mut e, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::settle(&mut e);
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
    assert_eq!(lk::pool_total(&e, lk::P1), 0);
}
