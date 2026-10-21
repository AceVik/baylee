//! Zur's Weirding — `{3}{U}` Enchantment (Ice Age)
//!
//! Players play with their hands revealed.
//! If a player would draw a card, they reveal it instead. Then any other player
//! may pay 2 life. If a player does, put that card into its owner's graveyard.
//! Otherwise, that player draws a card.
//! (`Coverage::Partial`: implements only `Modifier::RevealAllHands`.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn zur_s_weirding() -> CardIndex {
    card_index("c6eaa147-3566-43a9-999a-d58b877496f5")
}

#[test]
fn zur_s_weirding_reveals_all_players_hands_while_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[zur_s_weirding()])
        .battlefield(0, &[island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(!engine.state().hand_revealed(p0));
    assert!(!engine.state().hand_revealed(p1));

    let card = in_hand(&engine, p0, zur_s_weirding()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let weirding = on_battlefield(&engine, p0, zur_s_weirding()).unwrap();
    assert!(
        engine.state().hand_revealed(p0) && engine.state().hand_revealed(p1),
        "Zur's Weirding reveals both players' hands while on the battlefield"
    );

    bury(&mut engine, &[weirding]);
    assert!(!engine.state().hand_revealed(p0));
    assert!(!engine.state().hand_revealed(p1));
}
