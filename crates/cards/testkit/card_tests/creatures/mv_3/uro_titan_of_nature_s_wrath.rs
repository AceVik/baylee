//! `cards/creatures/mv_3/uro_titan_of_nature_s_wrath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cast from hand, Uro did not escape: its enters trigger sacrifices it,
/// and the enters-or-attacks trigger still gains 3 life, draws a card and
/// puts the Island from hand onto the battlefield.
#[test]
fn uro_cast_from_hand_grows_and_is_sacrificed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spare = in_hand(&engine, p0, island()).unwrap();
    cast_from_hand(&mut engine, p0, uro_titan_of_nature_s_wrath());
    resolve_uro(&mut engine, Some(spare));
    assert_eq!(engine.state().players[0].life, 23);
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "drew the library's Forest"
    );
    assert_eq!(
        engine.state().object(spare).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Island from hand"
    );
    assert!(
        in_graveyard(&engine, p0, uro_titan_of_nature_s_wrath()).is_some(),
        "sacrificed: it did not escape"
    );
    assert!(on_battlefield(&engine, p0, uro_titan_of_nature_s_wrath()).is_none());
}

/// Four other cards in the graveyard cannot pay an escape that exiles five:
/// Uro is not offered and a cast is refused, with its mana floating. A
/// fifth card makes the cast, whose only question after the price is which
/// five.
#[test]
fn uro_escapes_only_with_five_other_cards_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let uro = hand_to_graveyard(&mut engine, p0, uro_titan_of_nature_s_wrath());
    seed_graveyard(&mut engine, p0, 4);
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(!legal.castable.contains(&uro), "four others are not five");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: uro })
            .is_err()
    );
    seed_graveyard(&mut engine, p0, 1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&uro), "five others pay it");
    let exiled = escape_uro(&mut engine, p0, uro);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{G}}{{U}}{{U}}"
    );
    for card in &exiled {
        assert_eq!(
            engine.state().object(*card).map(|o| o.zone),
            Some(Zone::Exile),
            "the five went to exile as the cost"
        );
    }
    resolve_uro(&mut engine, None);
    assert_eq!(
        engine.state().object(uro).map(|o| o.zone),
        Some(Zone::Battlefield),
        "it escaped, so it stays"
    );
    assert_eq!(engine.state().players[0].life, 23);
}

/// Escape is a way to cast from the graveyard and nothing else: Uro in hand
/// is offered no escape, whatever lies in the graveyard, and costs its mana
/// cost with no question about exiling.
#[test]
fn uro_in_hand_is_cast_for_its_mana_cost_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 6);
    cast_from_hand(&mut engine, p0, uro_titan_of_nature_s_wrath());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no cast mode and no exile: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{G}}{{U}} of four"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        6,
        "nothing exiled"
    );
}

/// "Whenever Uro enters or attacks": an escaped Uro attacking gains 3 more,
/// draws, and may put a land; answered with none, the hand keeps its land.
#[test]
fn uro_attacking_grows_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let uro = hand_to_graveyard(&mut engine, p0, uro_titan_of_nature_s_wrath());
    seed_graveyard(&mut engine, p0, 5);
    tap_all_mana(&mut engine, p0);
    escape_uro(&mut engine, p0, uro);
    resolve_uro(&mut engine, None);
    assert_eq!(engine.state().players[0].life, 23);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.number > 1
            && e.state().turn.phase == Phase::FirstMain
    });
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&uro)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(uro, Defender::Player(p1))],
            },
        )
        .unwrap();
    resolve_uro(&mut engine, None);
    assert_eq!(engine.state().players[0].life, 26, "3 more for attacking");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a card drawn, no land put"
    );
    assert_eq!(
        engine.state().object(uro).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// Escaping belongs to the spell and the permanent it became, and to no
/// later object (CR 400.7). An escaped Uro blinked by Ephemerate comes back
/// a new permanent that did not escape, and its new enters trigger
/// sacrifices it.
#[test]
fn uro_escaped_and_blinked_is_sacrificed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), island(), plains()])
        .hand(0, &[uro_titan_of_nature_s_wrath(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let uro = hand_to_graveyard(&mut engine, p0, uro_titan_of_nature_s_wrath());
    seed_graveyard(&mut engine, p0, 5);
    let plains = on_battlefield(&engine, p0, plains()).unwrap();
    tap_mana_except(&mut engine, p0, plains);
    escape_uro(&mut engine, p0, uro);
    resolve_uro(&mut engine, None);
    assert_eq!(
        engine.state().object(uro).map(|o| o.zone),
        Some(Zone::Battlefield),
        "escaped, so it stayed"
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ephemerate());
    let _ = aim_at(&mut engine, p0, uro);
    resolve_uro(&mut engine, None);
    assert_eq!(
        engine.state().object(uro).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the blinked Uro did not escape"
    );
}
