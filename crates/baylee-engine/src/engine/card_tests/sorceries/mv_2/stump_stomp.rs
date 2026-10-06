//! `cards/sorceries/mv_2/stump_stomp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Stump Stomp` // `Burnwillow Clearing`: "Target creature you control deals
/// damage equal to its power to target creature or planeswalker you don't
/// control. // This land enters tapped. {T}: Add {R} or {G}."
///
/// The back face: the test plays the land face, verifies it enters tapped,
/// advances to the next turn so it untaps, and activates its mana ability
/// choosing `{G}` from `Pending::ChooseColor`. The front face is the two
/// tests below.
#[test]
fn burnwillow_clearing_enters_tapped_and_taps_for_chosen_mana() {
    let (mut engine, land) =
        play_land_face(stump_stomp(), 1).expect("plays as Burnwillow Clearing");
    assert!(
        is_tapped(&engine, land),
        "Burnwillow Clearing enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);

    activate(&mut engine, p0, stump_stomp(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "offers Red and Green"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green_before + 1,
        "adds one green mana to the pool"
    );
}

/// Stump Stomp, the front face, at a creature: my 4/4 deals four to their 2/2
/// and is dealt nothing back, because this is one-sided and not a fight.
#[test]
fn stump_stomp_has_my_creature_deal_its_power_and_take_nothing_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2190, forest())
        .battlefield(0, &[forest(), mountain(), fangren_hunter()])
        .battlefield(1, &[wild_colos()])
        .hand(0, &[stump_stomp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, stump_stomp());
    for chosen in [hunter, colos] {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![chosen],
                    players: vec![],
                },
            )
            .expect("each creature is on its own menu");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "four damage kill the 2/2"
    );
    assert_eq!(
        engine.state().object(hunter).map(|o| o.damage),
        Some(0),
        "nothing is dealt back"
    );
}

/// Stump Stomp at a planeswalker: "target creature **or planeswalker** you
/// don't control". Damage to a planeswalker removes that much loyalty
/// (CR 306.8) — a 4/4 takes Karn from five to one — and my own creatures are
/// not on the second menu at all.
#[test]
fn stump_stomp_can_hit_a_planeswalker_and_takes_its_loyalty() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2191, forest())
        .battlefield(
            0,
            &[forest(), mountain(), fangren_hunter(), llanowar_elves()],
        )
        .battlefield(1, &[karn_the_great_creator()])
        .hand(0, &[stump_stomp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("their Karn is out");
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, stump_stomp());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .expect("my Hunter deals the damage");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the second target question, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&karn),
        "a planeswalker I don't control is a target"
    );
    assert!(
        !options.contains(&elves) && !options.contains(&hunter),
        "and nothing of mine is"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .expect("Karn was offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(karn)
            .map(|o| o.counters.get(CounterKind::Loyalty)),
        Some(1),
        "five loyalty less four damage"
    );
}
