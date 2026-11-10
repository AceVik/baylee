//! `cards/lands/utility/contested_cliffs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Contested Cliffs' mana half, `{T}: Add {C}`; the fight half is the test
/// below. This plays it for real: the land arrives through `PlayLand` rather
/// than `starting_battlefield`, and then taps, with a Forest beside it as the
/// control — one colourless and one green is the only pool that says the {C}
/// came off the Cliffs and not off a land that was already making mana.
#[test]
fn contested_cliffs_is_played_as_a_land_and_taps_for_one_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[contested_cliffs()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let cliffs = play_land(&mut engine, p0, contested_cliffs());
    assert!(
        !is_tapped(&engine, cliffs),
        "it enters untapped; the mana ability is what turns it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land floats nothing"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 2,
        "the Cliffs' printed {{T}}: Add {{C}} — the half of #159 that the \
         305.6 shortcut cannot see — and the Forest's basic land type are the \
         two sources on this board"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the one mana the Cliffs prints, and it is colourless"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "beside the Forest's green, which is the control"
    );
    assert_eq!(pool.total(), 2, "two sources, two mana, nothing else");
    assert!(is_tapped(&engine, cliffs), "the Cliffs paid its own {{T}}");
    assert!(is_tapped(&engine, land), "and so did the land beside it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}

/// Contested Cliffs' fight: "{R}{G}, {T}: Target **Beast** creature you
/// control fights target creature an **opponent** controls."
///
/// An activated ability asks both target questions before its cost is paid
/// (CR 602.2b follows CR 601.2c–h). The first menu holds my Beast and not my
/// Elves; the second holds their creature and nothing of mine. Then the 4/4
/// Beast fights the 2/2.
#[test]
fn contested_cliffs_makes_my_beast_fight_an_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                mountain(),
                contested_cliffs(),
                fangren_hunter(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[wild_colos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Beast is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    let cliffs = on_battlefield(&engine, p0, contested_cliffs()).expect("the Cliffs are out");

    tap_mana_where(&mut engine, p0, |id| id != cliffs && id != elves);
    activate(&mut engine, p0, contested_cliffs(), 1);

    let first = aim_at(&mut engine, p0, hunter);
    assert!(first.contains(&hunter), "a Beast creature I control");
    assert!(!first.contains(&elves), "an Elf is not a Beast");
    assert!(!first.contains(&colos), "and theirs is not mine");
    let second = aim_at(&mut engine, p0, colos);
    assert_eq!(second, vec![colos], "only a creature an opponent controls");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, cliffs), "{{T}} was paid");
    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "four damage kill the 2/2"
    );
    assert_eq!(
        engine.state().object(hunter).map(|o| o.damage),
        Some(2),
        "and the Beast took two"
    );
}
