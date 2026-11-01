//! `cards/enchantments/auras/mv_2/cunning.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cunning() -> CardIndex {
    card_index("d5e48041-5f21-48e6-bd0c-000f02dfbe8a")
}

/// Cunning (`d5e48041-5f21-48e6-bd0c-000f02dfbe8a`) — {1}{U} Enchantment — Aura (Implemented):
/// "Enchant creature. Enchanted creature gets +3/+3. When enchanted creature attacks or blocks, sacrifice this Aura at the beginning of the next cleanup step."
#[test]
fn cunning_grants_plus_3_plus_3_and_sacrifices_itself_at_next_cleanup_when_enchanted_creature_attacks()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), index::GRIZZLY_BEARS])
        .hand(0, &[cunning()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, cunning());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (5, 5));

    kit::declare_attack(&mut engine, &[bear], P1);
    pass_until(&mut engine, |e| e.state().turn.active == P1);

    assert!(on_battlefield(&engine, P0, cunning()).is_none());
    assert!(in_graveyard(&engine, P0, cunning()).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(pt(&engine, bear), (2, 2));
}

#[test]
fn cunning_on_an_opponents_blocker_sacrifices_the_aura_and_leaves_the_host() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[cunning()],
        &[],
    );
    let attacker = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let host = lk::obj(&e, P1, index::HILL_GIANT);
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut e, P0, cunning(), &[host], &[]);
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(host, attacker)]);
    reach_their_main_phase(&mut e, P1);
    assert!(in_graveyard(&e, P0, cunning()).is_some());
    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_some());
    assert_eq!(pt(&e, host), (3, 3));
}
#[test]
fn cunning_stays_when_its_creature_did_not_attack_or_block() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[cunning()], &[]);
    let host = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut e, P0, cunning(), &[host], &[]);
    reach_their_main_phase(&mut e, P1);
    assert!(on_battlefield(&e, P0, cunning()).is_some());
    assert_eq!(pt(&e, host), (5, 5));
}
