//! `cards/creatures/mv_7/charnelhoard_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn charnelhoard_wurm() -> CardIndex {
    card_index("4a430fa3-e693-424b-9981-d7d8193445e3")
}

/// Charnelhoard Wurm — a 6/6 trampler for {4}{B}{R}{G}.
#[test]
fn charnelhoard_wurm_costs_seven_in_three_colors() {
    let board = [lots(swamp(), 2), lots(mountain(), 2), lots(forest(), 3)].concat();
    let mut engine = table7(&board, &[charnelhoard_wurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, charnelhoard_wurm());
    assert_body(&engine, wurm, (6, 6), KeywordSet::TRAMPLE);
}

/// "Whenever this creature deals damage to an opponent, you may return target
/// card from your graveyard to your hand." A card put in the graveyard first
/// is the only target; answering yes returns it.
#[test]
fn charnelhoard_wurm_returns_a_card_when_it_hits() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[charnelhoard_wurm()], &[], &[]);
    seed_graveyard(&mut engine, p0, 1);
    let card = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .first()
        .copied()
        .expect("one card in the graveyard");
    let wurm = on_battlefield(&engine, p0, charnelhoard_wurm()).expect("out");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    declare_attack(&mut engine, &[wurm], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let (options, _) = target_menu(&engine);
    assert_eq!(
        options,
        vec![card],
        "the graveyard's one card is the target"
    );
    point_at(&mut engine, p0, &[card], &[]);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 14, "6 damage to the opponent");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the card came back to the hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
    let _ = p1;
}
