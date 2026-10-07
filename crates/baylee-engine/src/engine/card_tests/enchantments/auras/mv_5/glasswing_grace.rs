//! `cards/enchantments/auras/mv_5/glasswing_grace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// CR 704.5m: an Aura attached to an illegal permanent — or to nothing at all
/// — is put into its owner's graveyard as a state-based action.
///
/// Exiling the host is the cleanest way to ask it, because nothing else in
/// the scenario touches the Aura: it is on the battlefield, its creature
/// leaves, and the only rule that can move it is that one. An Aura that
/// stayed would go on granting +2/+2 to an object that is no longer there.
#[test]
fn a_glasswing_grace_falls_into_the_graveyard_when_its_creature_is_exiled() {
    let p0 = PlayerId::new(0);
    let (mut engine, mine, _theirs, aura) = a_glasswing_on_the_elves();

    // Back to an own first main phase — the phase is named rather than left
    // to `at_rest`, which is already true in turn three's upkeep — where five
    // Plains have untapped and the Swords left in hand is castable.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && at_rest(e, p0)
    });
    assert!(
        engine
            .state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the Aura is still on the battlefield before the removal"
    );

    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mine),
        "the enchanted creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .object(mine)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "the host was exiled"
    );
    assert!(
        on_battlefield(&engine, p0, glasswing_grace()).is_none(),
        "the Aura did not stay on the battlefield with nothing to enchant"
    );
    assert!(
        in_graveyard(&engine, p0, glasswing_grace()).is_some(),
        "an Aura enchanting nothing goes to its owner's graveyard (CR 704.5m)"
    );
}

/// CR 303.4c: "illegal" is the **enchant ability's** word, not "gone". An
/// Aura whose host is still on the battlefield, and still a permanent, but
/// has stopped being something that Aura may enchant is put into its
/// owner's graveyard all the same.
///
/// Swift Reconfiguration is the pool's one card that takes creaturehood
/// away without taking the permanent, so it is the only way to ask the
/// question at all: Glasswing Grace says "enchant creature", and after the
/// Reconfiguration resolves its host is an artifact Vehicle and no creature.
/// The test asserts both halves, because a rule that swept every Aura off
/// the table would pass the first — Swift Reconfiguration enchants "creature
/// or Vehicle" and stays, over exactly the host that has just cost the other
/// Aura its place.
#[test]
fn an_aura_falls_off_a_host_that_stops_being_what_it_enchants() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(517, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[glasswing_grace(), swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");

    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, glasswing_grace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    let aura = on_battlefield(&engine, p0, glasswing_grace()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and is attached to the Elves"
    );
    assert_eq!(pt(&engine, host), (3, 3), "a 1/1 under +2/+2");

    // The sixth Plains is still untapped: Glasswing Grace costs five.
    cast_from_hand(&mut engine, p0, swift_reconfiguration());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let now = types(&engine, host);
    assert!(
        now.contains(TypeSet::ARTIFACT) && !now.contains(TypeSet::CREATURE),
        "the host is an artifact Vehicle and no longer a creature: {now:?}"
    );
    assert!(
        on_battlefield(&engine, p0, glasswing_grace()).is_none()
            && in_graveyard(&engine, p0, glasswing_grace()).is_some(),
        "so the Aura that says `enchant creature` is in its owner's \
         graveyard (CR 303.4c, CR 704.5m)"
    );
    assert_eq!(
        on_battlefield(&engine, p0, swift_reconfiguration())
            .and_then(|a| engine.state().object(a))
            .and_then(|o| o.attached_to),
        Some(host),
        "while the Aura that says `enchant creature or Vehicle` is still on \
         the battlefield and still attached to the same permanent"
    );
}

/// The back face. "Age-Graced Chapel — Land. This land enters tapped.
/// {T}: Add {W} or {B}." CR 712.12: a player playing a modal double-faced
/// card as a land chooses one of its faces that's a land — it is *played*
/// as a land drop rather than cast, and what arrives is that land, with
/// none of the Aura's printed statics on it.
///
/// The pool-wide land sweep cannot speak for this card: `land_mana_tests`
/// sweeps only the faces that arrive untapped and counts the rest, and this
/// one is exactly that.
#[test]
fn age_graced_chapel_is_played_as_a_tapped_land_that_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, chapel) =
        play_land_face(glasswing_grace(), 1).expect("the back face is a land and can be played");

    let kinds = types(&engine, chapel);
    assert!(
        kinds.contains(TypeSet::LAND),
        "the face that was played is the land"
    );
    assert!(
        !kinds.contains(TypeSet::ENCHANTMENT),
        "and it is not the Aura on the other side"
    );
    assert!(
        entered_tapped(&engine, chapel),
        "\"This land enters tapped.\""
    );

    // Its own untap step gives it back, which is the first moment the {T} in
    // the ability's cost is payable at all.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && at_rest(e, p0)
    });
    assert!(
        !is_tapped(&engine, chapel),
        "the untap step untapped the Chapel"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&chapel),
        "the Chapel prints no basic land type, so there is no CR 305.6 shortcut"
    );
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == chapel)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        vec![0],
        "the land face offers its own mana ability and nothing the Aura printed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: chapel,
                ability_index: 0,
            },
        )
        .expect("an untapped land activates its own {T} ability");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Black],
        "the two colours the Chapel prints, and only those"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black came out of its own list");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "one black mana in the pool"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and nothing of the colour that was not chosen"
    );
    assert!(
        is_tapped(&engine, chapel),
        "the {{T}} in the ability's cost"
    );
}
