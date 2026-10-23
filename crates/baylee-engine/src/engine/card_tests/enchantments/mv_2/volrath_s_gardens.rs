//! `cards/enchantments/mv_2/volrath_s_gardens.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn volrath_s_gardens() -> CardIndex {
    card_index("ff99337e-410c-43af-9475-7995d690bcef")
}

/// Volrath's Gardens (`ff99337e-410c-43af-9475-7995d690bcef`) — {1}{G} Enchantment (Implemented):
/// "{2}, Tap an untapped creature you control: You gain 2 life. Activate only as a sorcery."
#[test]
fn volrath_s_gardens_taps_creature_to_gain_two_life_at_sorcery_speed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                volrath_s_gardens(),
                index::GRIZZLY_BEARS,
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gardens = on_battlefield(&engine, P0, volrath_s_gardens()).unwrap();
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    kit::activate_object(&mut engine, P0, gardens, 0);
    kit::settle(&mut engine, P0, |_, p| kit::choose_cards(p, &[bear]));

    assert!(entered_tapped(&engine, bear));
    assert_eq!(engine.state().players[0].life, 22);
}

#[test]
fn tapped_or_opposing_creatures_cannot_pay_and_activation_is_sorcery_speed() {
    let mut e = lk::game(
        SEED,
        &[volrath_s_gardens(), index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 10)]);
    assert!(lk::offered(&e, volrath_s_gardens(), P0, 0));
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    e.refresh_offer();
    assert!(
        !lk::offered(&e, volrath_s_gardens(), P0, 0),
        "opposing untapped creature cannot pay"
    );
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .status
        .remove(Status::TAPPED);
    e.refresh_offer();
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 10)]);
    assert!(!lk::offered(&e, volrath_s_gardens(), P0, 0));
}
