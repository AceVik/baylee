//! `cards/instants/mv_4/blind_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{WAR_MAMMOTH, WIND_DRAKE};

fn blind_fury() -> CardIndex {
    card_index("5dbfd316-a0a9-4caa-99b1-069931d2aaa4")
}

#[test]
fn blind_fury_removes_trample_from_all_creatures_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[blind_fury()])
        .battlefield(
            0,
            &[WAR_MAMMOTH, mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mammoth = on_battlefield(&engine, p0, WAR_MAMMOTH).unwrap();
    assert!(
        engine
            .state()
            .object(mammoth)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );

    cast_from_hand(&mut engine, p0, blind_fury());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(mammoth)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "Blind Fury should remove trample from all creatures until end of turn"
    );
}

#[test]
fn blind_fury_strips_only_trample_from_every_creature_and_only_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[blind_fury()])
        .battlefield(
            0,
            &[
                WAR_MAMMOTH,
                WIND_DRAKE,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[WAR_MAMMOTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, WAR_MAMMOTH).unwrap();
    let theirs = on_battlefield(&engine, p1, WAR_MAMMOTH).unwrap();
    let drake = on_battlefield(&engine, p0, WIND_DRAKE).unwrap();
    cast_from_hand(&mut engine, p0, blind_fury());
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, mine).contains(KeywordSet::TRAMPLE));
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "all creatures, the opponent's too"
    );
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "only trample is lost"
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "until end of turn"
    );
}
