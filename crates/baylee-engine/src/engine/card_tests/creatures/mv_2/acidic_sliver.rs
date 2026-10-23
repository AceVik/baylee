//! `cards/creatures/mv_2/acidic_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn acidic_sliver() -> CardIndex {
    card_index("a149eeb1-007c-435d-9d55-34af205090e0")
}

/// Acidic Sliver (`a149eeb1-007c-435d-9d55-34af205090e0`) — {B}{R} 2/2 Creature — Sliver (Implemented):
/// "All Slivers have '{2}, Sacrifice this permanent: This permanent deals 2 damage to any target.'"
#[test]
fn acidic_sliver_grants_sacrifice_two_damage_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), mountain(), forest(), forest()])
        .hand(0, &[acidic_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, acidic_sliver());
    pass_until(&mut engine, stack_is_empty);

    let sliver = on_battlefield(&engine, P0, acidic_sliver()).unwrap();
    tap_all_mana(&mut engine, P0);
    let idx = kit::offered(&engine, sliver)[0];
    kit::activate_object(&mut engine, P0, sliver, idx);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, acidic_sliver()).is_some());
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn opponent_sliver_gets_the_activation_but_a_nonsliver_does_not() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[acidic_sliver()],
        &[index::METALLIC_SLIVER, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    reach_their_main_phase(&mut e, P1);
    lk::float(&mut e, P1, &[(ManaColor::Colorless, 2)]);
    let sliver = lk::obj(&e, P1, index::METALLIC_SLIVER);
    let bear = lk::obj(&e, P1, index::GRIZZLY_BEARS);
    assert!(kit::offered(&e, bear).is_empty());
    let ability = kit::offered(&e, sliver)[0];
    kit::activate_object(&mut e, P1, sliver, ability);
    lk::aim(&mut e, &[], &[P0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 18);
    assert!(in_graveyard(&e, P1, index::METALLIC_SLIVER).is_some());
    assert!(on_battlefield(&e, P0, acidic_sliver()).is_some());
}
