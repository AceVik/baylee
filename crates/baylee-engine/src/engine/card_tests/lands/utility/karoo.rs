//! `cards/lands/utility/karoo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The same question, declined: naming nothing is how a player says no, and
/// what follows is the sacrifice the card prints.
///
/// `min: 0` is the whole of that — an empty answer has to be *legal*, or the
/// only way out of the question would be to pay. Written against Karoo
/// because one row is enough for the branch: the fallback is the same
/// `Effect::SacrificeSelf` on all ten.
#[test]
fn a_land_that_costs_a_bounce_sacrifices_itself_when_the_player_declines() {
    let p0 = PlayerId::new(0);
    let karoo = card_index("d4e875d9-2245-470d-aa2f-1dfe66ce2d15");
    let mut engine = Duel::new(921, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[karoo])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, karoo);
    let (options, _) =
        reach_the_unless_question(&mut engine, land).expect("the land asks what pays");
    let plain = options[0];

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "declining sacrifices the land"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&plain),
        "and the Plains that was offered stays where it was"
    );
}

/// Nobody can pay, so nobody is asked.
///
/// The Plains is **tapped**, which is the word the Karoo cycle prints and
/// the one half of its filter a test on an empty board would not reach: a
/// land on the battlefield that cannot pay is a menu with nothing on it, and
/// the engine runs the fallback without putting a question up at all. A
/// prompt with no legal answer would be a dead end for a client.
#[test]
fn a_land_that_costs_a_bounce_nobody_can_pay_asks_nothing() {
    let p0 = PlayerId::new(0);
    let karoo = card_index("d4e875d9-2245-470d-aa2f-1dfe66ce2d15");
    let mut engine = Duel::new(922, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[karoo])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Tap the Plains for mana, which is exactly how a player arrives here.
    let plain = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == plains())
        })
        .expect("the Plains is on the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plain })
        .unwrap();

    let land = play_land(&mut engine, p0, karoo);
    assert!(
        reach_the_unless_question(&mut engine, land).is_none(),
        "an untapped Plains is what the card asks for, and there is none"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "so the land sacrifices itself"
    );
}
