//! `cards/lands/utility/forsaken_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forsaken City prints `This land doesn't untap during your untap step`, `At the beginning of your
/// upkeep, you may exile a card from your hand. If you do, untap this land`, and `{T}: Add one mana of any color.`
/// The card is marked `Coverage::Partial` because exiling from hand during upkeep to untap is unsupported.
/// Forsaken City enters untapped, activates ability index 1 to add chosen mana, and `Modifier::DoesNotUntap`
/// keeps it tapped through subsequent untap steps.
#[test]
fn forsaken_city_adds_any_color_and_does_not_untap_during_untap_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forsaken_city()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, forsaken_city());
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, forsaken_city(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(is_tapped(&engine, land));
}
