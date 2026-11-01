#[allow(clippy::wildcard_imports)]
use super::*;

fn torch_song() -> CardIndex {
    card_index("69826a0d-3ec9-47e7-b129-2fc13b41a07e")
}

#[test]
fn torch_song_adds_verse_counter_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[torch_song(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let song_id = on_battlefield(&engine, p0, torch_song()).expect("song on bf");
    assert_eq!(
        engine
            .state()
            .object(song_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, torch_song(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn torch_song_declined_counter_and_zero_damage_do_not_become_a_free_point() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game_with(SEED, &[torch_song()], &[], &[], &[], |e| {
        lk::yes_no(e, false);
    });
    let song = lk::obj(&e, p0, torch_song());
    reach_their_main_phase(&mut e, p1);
    assert_eq!(counters_on(&e, song, baylee_cards_dsl::counters::VERSE), 0);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(&mut e, p0, &[(ManaColor::Red, 3)]);
    lk::use_ability(&mut e, p0, torch_song(), 1, &[], &[p1]);
    assert_eq!(lk::life(&e, p1), 20);
    assert!(in_graveyard(&e, p0, torch_song()).is_some());
}
