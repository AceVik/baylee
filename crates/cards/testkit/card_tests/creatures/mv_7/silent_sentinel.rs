//! `cards/creatures/mv_7/silent_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn silent_sentinel() -> CardIndex {
    card_index("26396b80-e5b7-44f5-916c-56a9215a7a49")
}

/// Silent Sentinel — a 4/6 flier for {5}{W}{W}.
#[test]
fn silent_sentinel_costs_seven_and_is_a_four_six_flier() {
    let mut engine = table7(&lots(plains(), 7), &[silent_sentinel()], &[]);
    let sentinel = cast_and_resolve(&mut engine, silent_sentinel());
    assert_body(&engine, sentinel, (4, 6), KeywordSet::FLYING);
}

/// "Whenever this creature attacks, you may return target enchantment card
/// from your graveyard to the battlefield." The library is all Luminarch
/// Ascensions, one of which is put in the graveyard first; attacking, the
/// Sentinel offers it as the only target and answering yes returns it.
#[test]
fn silent_sentinel_returns_an_enchantment_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, luminarch_ascension())
        .battlefield(0, &[silent_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let card = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .first()
        .copied()
        .expect("one card in the graveyard");
    let sentinel = on_battlefield(&engine, p0, silent_sentinel()).expect("out");
    assert!(on_battlefield(&engine, p0, luminarch_ascension()).is_none());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sentinel, Defender::Player(PlayerId::new(1)))],
            },
        )
        .expect("the Sentinel attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let (options, _) = target_menu(&engine);
    assert_eq!(options, vec![card], "the enchantment card in the graveyard");
    point_at(&mut engine, p0, &[card], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(
        on_battlefield(&engine, p0, luminarch_ascension()),
        Some(card),
        "back on the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
}
