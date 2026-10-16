//! `cards/artifacts/mv_4/five_hundred_year_diary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn five_hundred_year_diary() -> CardIndex {
    card_index("9bdd6fea-d436-412b-a7b7-bbdba73dc986")
}

/// A second Clue: Candy Trail is a Food Clue artifact.
fn candy_trail() -> CardIndex {
    card_index("7c8e4f29-53e0-4242-8ef8-7a9476dce178")
}

/// "Five Hundred Year Diary enters tapped."
#[test]
fn the_diary_enters_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 4])
        .hand(0, &[five_hundred_year_diary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, five_hundred_year_diary());
    pass_until(&mut engine, stack_is_empty);
    let diary = on_battlefield(&engine, p0, five_hundred_year_diary()).expect("resolved");
    assert!(is_tapped(&engine, diary), "it entered tapped");
    assert!(
        !priority_offer(&engine).mana_abilities.contains(&diary),
        "a tapped Diary offers no mana"
    );
}

/// "{T}: Add {U} for each Clue you control." The Diary is itself a Clue,
/// and so is the Candy Trail beside it; an opposing Clue would not count.
#[test]
fn the_diary_adds_one_blue_for_each_clue_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mana = |mine: &[CardIndex], theirs: &[CardIndex]| {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, mine)
            .battlefield(1, theirs)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let diary = on_battlefield(&engine, p0, five_hundred_year_diary()).unwrap();
        tap_mana_where(&mut engine, p0, |id| id == diary);
        assert!(is_tapped(&engine, diary));
        let pool = &engine.state().players[0].mana_pool;
        (pool.available(ManaColor::Blue), pool.total())
    };
    assert_eq!(mana(&[five_hundred_year_diary()], &[]), (1, 1), "itself");
    assert_eq!(
        mana(&[five_hundred_year_diary(), candy_trail()], &[]),
        (2, 2),
        "a second Clue of mine"
    );
    let _ = p1;
    assert_eq!(
        mana(&[five_hundred_year_diary()], &[candy_trail()]),
        (1, 1),
        "their Clue does not count"
    );
}

/// "{2}, Sacrifice Five Hundred Year Diary: Draw a card."
#[test]
fn sacrificing_the_diary_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[five_hundred_year_diary(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_except(
        &mut engine,
        p0,
        on_battlefield(&engine, p0, five_hundred_year_diary()).unwrap(),
    );
    activate(&mut engine, p0, five_hundred_year_diary(), 1);
    assert!(in_graveyard(&engine, p0, five_hundred_year_diary()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
}
