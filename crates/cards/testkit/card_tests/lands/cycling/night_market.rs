//! `cards/lands/cycling/night_market.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Night Market` enters tapped, prompts for a color as it enters, and features
/// cycling `{{3}}` under `Coverage::Implemented`.
/// Playing one copy prompts for a color choice via `Pending::ChooseColor` and leaves it tapped,
/// while a second copy in hand discards itself to cycle and draws a card.
#[test]
fn night_market_enters_tapped_chooses_color_and_cycles_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1903, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[night_market(), night_market()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = in_hand(&engine, p0, night_market()).expect("first copy in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: first })
        .unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt on entry, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 5);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert!(entered_tapped(&engine, first));

    let second = in_hand(&engine, p0, night_market()).expect("second copy in hand");
    assert_ne!(first, second);

    let library_before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, night_market(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, night_market()).is_some());
    assert_eq!(library_size(&engine, p0), library_before - 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
