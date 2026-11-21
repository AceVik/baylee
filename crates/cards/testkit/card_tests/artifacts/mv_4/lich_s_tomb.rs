//! `cards/artifacts/mv_4/lich_s_tomb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lich_s_tomb() -> CardIndex {
    card_index("600b9ab8-36dc-4f02-b48c-091348d132ac")
}

/// Bolts p0 from p1 using `mtn` and answers Lich's Tomb's sacrifice trigger,
/// keeping `tomb` as long as enough other permanents remain.
#[track_caller]
fn bolt_p0_and_sacrifice(engine: &mut Engine<RegistryLookup>, mtn: ObjectId, tomb: ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    tap_mana_where(engine, p1, |id| id == mtn);
    cast_with_floating(engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::GameOver(_)
        ) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0);
        assert_eq!(min, max, "must choose exact count to keep");
        // Simultaneous selection asks which permanents to KEEP (total - life_lost).
        let keep = usize::from(min);
        let mut chosen = Vec::with_capacity(keep);
        if keep > 0 && options.contains(&tomb) {
            chosen.push(tomb);
        }
        for id in options {
            if chosen.len() >= keep {
                break;
            }
            if id != tomb {
                chosen.push(id);
            }
        }
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
            .unwrap();
        pass_until(engine, |e| {
            matches!(e.pending(), Pending::GameOver(_)) || stack_is_empty(e)
        });
    }
}

/// Lich's Tomb (600b9ab8-36dc-4f02-b48c-091348d132ac) — {4} Artifact:
/// "You don't lose the game for having 0 or less life."
/// "Whenever you lose life, sacrifice a permanent for each 1 life you lost."
///
/// Seven Bolts deal 21 damage (life 20 -> -1) and sacrifice all 21 Forests;
/// p0 stays alive at -1 life with Lich's Tomb still in play, then an eighth
/// Bolt forces sacrificing Lich's Tomb itself and immediately ends the game.
#[test]
fn lich_s_tomb_sacrifices_for_life_lost_and_prevents_zero_life_loss_while_present() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut p0_board = vec![lich_s_tomb()];
    p0_board.resize(22, forest());
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &p0_board)
        .battlefield(1, &[mountain(); 8])
        .hand(1, &[lightning_bolt(); 8])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let tomb = on_battlefield(&engine, p0, lich_s_tomb()).unwrap();
    let p1_mtns = lands_of(&engine, p1);

    // 1. First Bolt (20 -> 17): forces sacrificing 3 Forests (18 Forests + Tomb remain).
    bolt_p0_and_sacrifice(&mut engine, p1_mtns[0], tomb);
    assert_eq!(life_of(&engine, p0), 17);
    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 18);

    // 2. Six more Bolts (17 -> -1): sacrifice remaining 18 Forests; p0 survives at -1 life!
    for &mtn in &p1_mtns[1..7] {
        bolt_p0_and_sacrifice(&mut engine, mtn, tomb);
    }
    assert_eq!(life_of(&engine, p0), -1);
    assert!(all_on_battlefield(&engine, p0, forest()).is_empty());
    assert!(on_battlefield(&engine, p0, lich_s_tomb()).is_some());
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "Lich's Tomb prevents losing at -1 life"
    );

    // 3. Eighth Bolt (-1 -> -4): Lich's Tomb is the only permanent left and is sacrificed -> loss!
    bolt_p0_and_sacrifice(&mut engine, p1_mtns[7], tomb);
    assert_eq!(on_battlefield(&engine, p0, lich_s_tomb()), None);
    assert!(
        matches!(engine.pending(), Pending::GameOver(_)),
        "once Lich's Tomb leaves at <= 0 life, p0 loses the game"
    );
}
