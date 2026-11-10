//! `cards/lands/utility/tarnation_vista.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

#[test]
fn tarnation_vista_enters_tapped_chooses_color_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tarnation_vista()])
        .battlefield(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, tarnation_vista()).expect("tarnation vista in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, baylee_cards_dsl::ALL_MANA_COLORS.to_vec());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let vista = on_battlefield(&engine, p0, tarnation_vista()).expect("vista on battlefield");
    assert!(entered_tapped(&engine, vista));

    // Advance to p0's next main phase to untap.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, vista));

    // Float {{1}} from Forest while keeping Tarnation Vista untapped.
    tap_mana_except(&mut engine, p0, vista);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, vista));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(vista, 0)),
        "ability 0 ({{T}}: Add chosen color) is offered"
    );
    assert!(
        legal.abilities.contains(&(vista, 1)),
        "ability 1 is offered with its generic cost available"
    );

    activate(&mut engine, p0, tarnation_vista(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, vista));
}

#[test]
fn vista_excludes_multicolored_permanents_and_counts_each_monocolor_once() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
                gray_ogre(),
                baleful_strix(),
            ],
        )
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[tarnation_vista()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let vista = in_hand(&e, p0, tarnation_vista()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card: vista }).unwrap();
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    e.dev_state_mut(p0).unwrap().set_tapped(vista, false);

    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, p0, tarnation_vista(), 1);
    assert!(stack_is_empty(&e));
    let pool = &e.state().players[0].mana_pool;
    assert_eq!(pool.total(), 2);
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
}
