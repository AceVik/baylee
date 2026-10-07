//! `cards/lands/utility/oscorp_industries.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oscorp Industries prints `This land enters tapped.`, `When this land enters from a graveyard, you lose 2 life.`, `{{T}}: Add {{U}}, {{B}}, or {{R}}.`, and `Mayhem (You may play this card from your graveyard if you discarded it this turn. Timing rules still apply.)`
///
/// Under `Coverage::Partial`, the enters-from-graveyard life loss trigger and the Mayhem mechanic are omitted because neither tracking entry origins nor graveyard discard permissions are supported.
/// Playing Oscorp Industries from hand enters the battlefield tapped without life loss. In the next turn, after it untaps, activating ability 0 prompts for a color choice between `ManaColor::Blue`, `ManaColor::Black`, and `ManaColor::Red` via `Pending::ChooseColor`, adds the chosen color to the mana pool, and leaves the land tapped.
#[test]
fn oscorp_industries_enters_tapped_and_produces_chosen_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[oscorp_industries()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let oscorp = play_land(&mut engine, p0, oscorp_industries());
    assert!(
        entered_tapped(&engine, oscorp),
        "oscorp industries enters tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "playing from hand causes no life loss"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, oscorp));

    activate(&mut engine, p0, oscorp_industries(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![ManaColor::Blue, ManaColor::Black, ManaColor::Red]
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, oscorp));
    assert_eq!(engine.state().players[0].life, 20);
}
