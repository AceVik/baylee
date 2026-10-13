//! `cards/lands/utility/urban_retreat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Urban Retreat` prints `This land enters tapped.`, `{{T}}: Add {{G}}, {{W}}, or {{U}}.`, and `{{2}}, Return a tapped creature you control to its owner's hand: Put this card from your hand onto the battlefield. Activate only as a sorcery.`
///
/// Playing `Urban Retreat` from hand enters tapped. After untapping on the next turn, activating ability 0 prompts via `Pending::ChooseColor` between green, white, and blue, adds one blue mana, and leaves `Urban Retreat` tapped.
#[test]
fn urban_retreat_enters_tapped_and_taps_for_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[urban_retreat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let retreat = play_land(&mut engine, p0, urban_retreat());
    assert!(
        entered_tapped(&engine, retreat),
        "urban retreat enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, retreat));

    activate(&mut engine, p0, urban_retreat(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![ManaColor::Green, ManaColor::White, ManaColor::Blue]
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, retreat));
}

#[test]
fn urban_retreat_returns_only_a_tapped_friendly_creature_as_a_cost() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[urban_retreat()])
        .battlefield(0, &[index::GRIZZLY_BEARS, index::SERRA_ANGEL])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&e, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();
    let retreat = in_hand(&e, p0, urban_retreat()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(bear, true);
    state.set_tapped(other, true);
    state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, urban_retreat(), 1);
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![bear]);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().object(retreat).unwrap().zone, Zone::Hand);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(retreat).unwrap().zone, Zone::Battlefield);
    assert!(is_tapped(&e, retreat));
    assert_eq!(e.state().players[0].lands_played_this_turn, 0);
}

/// "{2}, Return a tapped creature you control to its owner's hand: Put this
/// card from your hand onto the battlefield. Activate only as a sorcery."
/// With a tapped creature of yours and the {2} floating, the ability is on
/// offer from the hand in the own main phase and in neither the stack-holding
/// window nor the opponent's turn.
#[test]
fn urban_retreat_hand_ability_is_sorcery_timing_only() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let mut e = kit::game(
        954,
        &[index::GRIZZLY_BEARS],
        &[],
        &[urban_retreat(), llanowar_elves()],
        &[],
    );
    let bear = kit::obj(&e, kit::P0, index::GRIZZLY_BEARS);
    e.dev_state_mut(kit::P0).unwrap().set_tapped(bear, true);
    let retreat = in_hand(&e, kit::P0, urban_retreat()).expect("Urban Retreat in hand");
    assert_sorcery_timing_only(&mut e, retreat, 1, &[(ManaColor::Colorless, 2)]);
}
