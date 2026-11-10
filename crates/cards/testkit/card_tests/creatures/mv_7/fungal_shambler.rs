//! `cards/creatures/mv_7/fungal_shambler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fungal_shambler() -> CardIndex {
    card_index("9991941e-436f-42d4-8b0c-1ee84234774b")
}

/// Fungal Shambler — a 6/4 trampler for {4}{B}{G}{U}.
#[test]
fn fungal_shambler_costs_seven_in_three_colors() {
    let board = [vec![swamp(), forest(), island()], lots(mountain(), 4)].concat();
    let mut engine = table7(&board, &[fungal_shambler()], &[]);
    let shambler = cast_and_resolve(&mut engine, fungal_shambler());
    assert_body(&engine, shambler, (6, 4), KeywordSet::TRAMPLE);
}

/// "Whenever this creature deals damage to an opponent, you draw a card and
/// that opponent discards a card." The opponent chooses what to discard.
#[test]
fn fungal_shambler_draws_for_me_and_discards_for_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fungal_shambler()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let shambler = on_battlefield(&engine, p0, fungal_shambler()).expect("out");
    let mine = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    declare_attack(&mut engine, &[shambler], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped here")
    };
    assert_eq!(player, p1, "the opponent chooses their discard");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: options })
        .expect("discarding their only card");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 14, "6 damage");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        mine + 1,
        "I drew a card"
    );
    assert!(
        in_graveyard(&engine, p1, island()).is_some(),
        "they discarded the Island"
    );
}
