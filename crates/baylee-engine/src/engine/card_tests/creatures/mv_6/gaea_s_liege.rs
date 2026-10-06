//! `cards/creatures/mv_6/gaea_s_liege.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gaea's Liege — {3}{G}{G}{G}, printed 0/0. Its "isn't attacking" half:
/// two Forests of its own, a Swamp beside them that must not count, and an
/// opponent's Forest that must not count either — read once, and again
/// after a third Forest is played as this turn's land, through the engine,
/// so the characteristic is shown tracking the count rather than a value
/// fixed once at entry. The "is attacking" half, which counts the
/// *defending* player's Forests instead, is played in the tests near the
/// end of this file.
#[test]
fn gaea_s_lieges_power_and_toughness_track_its_controllers_forests() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gaea_s_liege(), forest(), forest(), swamp()])
        .battlefield(1, &[forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let liege = on_battlefield(&engine, p0, gaea_s_liege()).expect("seated");
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the opponent has a Forest of their own, which must not count"
    );
    assert_eq!(
        pt(&engine, liege),
        (2, 2),
        "two Forests of its own; the Swamp beside them and the opponent's \
         Forest across the table do not count"
    );

    reach_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    assert_eq!(
        pt(&engine, liege),
        (3, 3),
        "a third Forest played as this turn's land, and the printed \
         characteristic followed the count up"
    );
}

/// Gaea's Liege's second sentence: "{T}: Target land becomes a Forest
/// until this creature leaves the battlefield." Aimed at a Swamp: it
/// gains the Forest type (which the first sentence's count reads too), it
/// can tap for {G} under the CR 305.6 shortcut a printed Swamp never had,
/// and once Gaea's Liege is destroyed by Hero's Downfall — leaving the
/// battlefield through the engine, not by rewriting the board — the
/// effect ends: the land is a Forest no longer, and is a Swamp again.
#[allow(clippy::too_many_lines)] // one target, one tap, one mana ability, one destroy spell
#[test]
fn gaea_s_lieges_second_ability_turns_a_land_into_a_forest_until_it_leaves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gaea_s_liege(), forest(), swamp(), swamp(), swamp()])
        .hand(0, &[hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let liege = on_battlefield(&engine, p0, gaea_s_liege()).expect("seated");
    let swamps = all_on_battlefield(&engine, p0, swamp());
    assert_eq!(swamps.len(), 3, "three Swamps are seated");
    let swamp_obj = swamps[0];
    let payment = [swamps[1], swamps[2]];
    assert_eq!(pt(&engine, liege), (1, 1), "one Forest so far");
    assert!(
        !engine
            .state()
            .object(swamp_obj)
            .expect("on the table")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST),
        "a Swamp is not a Forest before the ability resolves"
    );

    activate(&mut engine, p0, gaea_s_liege(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        options.contains(&swamp_obj),
        "the Swamp is a legal \"target land\": {options:?}"
    );
    assert!(
        !is_tapped(&engine, liege),
        "targets are chosen before the {{T}} cost is paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![swamp_obj],
                players: vec![],
            },
        )
        .expect("the Swamp was among the options the ability enumerated");
    assert!(is_tapped(&engine, liege), "the {{T}} was the price");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(swamp_obj)
            .expect("still on the table")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST),
        "\"becomes a Forest\": the target land now carries the type"
    );
    assert_eq!(
        pt(&engine, liege),
        (2, 2),
        "two Forests now, and the first sentence's count followed the change"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_obj })
        .expect("a Forest taps for green under CR 305.6, even one that used to be a Swamp");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the mana it made was green"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else floated"
    );

    // Destroy Gaea's Liege with a spell — through the engine, not by
    // rewriting the board — so the "until this creature leaves" effect
    // ends for the reason its own text names.
    tap_mana_where(&mut engine, p0, |id| payment.contains(&id));
    cast_with_floating(&mut engine, p0, hero_s_downfall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Hero's Downfall asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&liege),
        "the Liege is a legal \"target creature\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![liege],
            },
        )
        .expect("the Liege was among the options the spell enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, gaea_s_liege()).is_some(),
        "Hero's Downfall destroyed it"
    );
    assert!(
        !engine
            .state()
            .object(swamp_obj)
            .expect("the land itself did not leave")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST),
        "\"until this creature leaves the battlefield\": Gaea's Liege is gone, \
         and the land is a Forest no longer"
    );
    assert!(
        engine
            .state()
            .object(swamp_obj)
            .expect("still on the table")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::SWAMP),
        "the effect ending returns it to its own printed type, a Swamp"
    );
}

// ---------------------------------------------------------------------------
// Gaea's Liege.
// ---------------------------------------------------------------------------

/// Gaea's Liege's "is attacking" half: "As long as Gaea's Liege is
/// attacking, its power and toughness are each equal to the number of
/// Forests defending player controls." Its own controller holds two
/// Forests and the defending player holds five: once attackers are
/// declared, the Liege is a 5/5, not a 2/2, deals combat damage
/// accordingly, and reverts to its controller's count once combat ends.
#[test]
fn gaea_s_lieges_power_and_toughness_track_the_defending_players_forests_while_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gaea_s_liege(), forest(), forest()])
        .battlefield(1, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let liege = on_battlefield(&engine, p0, gaea_s_liege()).expect("seated");
    assert_eq!(
        pt(&engine, liege),
        (2, 2),
        "not attacking: its own controller's two Forests"
    );

    reach_main_phase(&mut engine, p0);
    attack_and_collect_blocks(&mut engine, liege, p1);
    assert_eq!(
        pt(&engine, liege),
        (5, 5),
        "attacking: the defending player's five Forests, not its \
         controller's two"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        15,
        "5 combat damage, matching the defending player's Forest count \
         while the Liege was attacking"
    );
    assert_eq!(
        pt(&engine, liege),
        (2, 2),
        "combat is over: back to its own controller's two Forests"
    );
}

/// Off the battlefield, neither of Gaea's Liege's sentences function: CR
/// 113.6 says an ability other than a spell's usually works only while its
/// object is on the battlefield, and lists several lettered exceptions
/// (113.6a through 113.6p). The exception that would matter here is
/// 113.6a, characteristic-defining abilities, which function everywhere —
/// but these two sentences are not CDAs: CR 604.3a's fifth criterion
/// excludes any ability that "does not set the values of such
/// characteristics only if certain conditions are met," which is exactly
/// what "as long as … isn't/is attacking" does. In hand, with three
/// Forests on the battlefield, the Liege stands at its printed 0/0.
#[test]
fn gaea_s_lieges_power_and_toughness_are_off_the_battlefield_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[gaea_s_liege()])
        .start();
    keep_mulligans(&mut engine);
    let liege = in_hand(&engine, p0, gaea_s_liege()).expect("the Liege sits in hand");
    assert_eq!(
        pt(&engine, liege),
        (0, 0),
        "its printed 0/0 stands off the battlefield despite three Forests"
    );
}
