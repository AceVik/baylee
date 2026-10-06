//! `cards/creatures/artifacts/mv_9/colossus_of_sardia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Colossus of Sardia — trample, "This creature doesn't untap during your
/// untap step", and "{9}: Untap this creature. Activate only during your
/// upkeep." Untapping is the turn-based action of the untap step (CR 502.3)
/// and the static removes this permanent from what it untaps: the 9/9 that
/// attacked is still tapped in its controller's next upkeep, having already
/// dealt its 9 with trample.
#[test]
fn colossus_of_sardia_stays_tapped_after_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[colossus_of_sardia()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let colossus = on_battlefield(&engine, p0, colossus_of_sardia()).expect("seated");
    assert_eq!(pt(&engine, colossus), (9, 9), "the printed body");
    assert!(
        keywords(&engine, colossus).contains(KeywordSet::TRAMPLE),
        "trample is the card's first line"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(colossus, Defender::Player(p1))],
            },
        )
        .expect("the Colossus attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, colossus), "attacking tapped it");
    assert_eq!(
        engine.state().players[1].life,
        11,
        "9 trample damage went through"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        is_tapped(&engine, colossus),
        "\"doesn't untap during your untap step\": the step has been and gone"
    );
}

/// The untap ability's window: "{9} … Activate only during your upkeep"
/// (CR 602.5). In the main phase, all the mana in the pool, the offer is
/// absent; in the controller's upkeep it appears only once nine are
/// floating, since the offer weighs the pool and not the untapped lands.
/// Paying the nine untaps the 9/9 the untap step left tapped, and — the
/// card's ruling — "there is no restriction on how many times it can be
/// untapped during your upkeep with this ability": nine more in the pool
/// offer it a second time.
#[allow(clippy::too_many_lines)] // one ability read at both of its boundaries
#[test]
fn colossus_of_sardia_untaps_for_nine_only_in_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![colossus_of_sardia()];
    board.extend_from_slice(&[forest(); 18]);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let colossus = on_battlefield(&engine, p0, colossus_of_sardia()).expect("seated");
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(colossus, 1));

    // A main phase with every Forest pooled: not the window.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 18);
    assert!(
        !offered(&engine),
        "the main phase is not the upkeep, eighteen mana or none"
    );

    // Tap it by attacking, so the ability has something to untap.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(colossus, Defender::Player(p1))],
            },
        )
        .expect("the Colossus attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, colossus));

    // Next upkeep, the pool empty first: mana still in the lands is no offer.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        !offered(&engine),
        "eighteen untapped Forests and an empty pool are not {{9}} attached"
    );

    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 18);
    tap_mana_where(&mut engine, p0, |id| forests[..8].contains(&id));
    assert_eq!(engine.state().players[0].mana_pool.total(), 8);
    assert!(!offered(&engine), "eight is not nine");
    tap_mana_where(&mut engine, p0, |id| id == forests[8]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 9);
    assert!(offered(&engine), "nine pays the printed {{9}}");

    activate(&mut engine, p0, colossus_of_sardia(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, colossus), "the ability untapped it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the nine were the price"
    );

    // The ruling: no restriction on how many times it untaps this upkeep.
    tap_mana_where(&mut engine, p0, |id| forests[9..].contains(&id));
    assert_eq!(engine.state().players[0].mana_pool.total(), 9);
    assert!(offered(&engine), "nine more offer it again");
    activate(&mut engine, p0, colossus_of_sardia(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, colossus));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
