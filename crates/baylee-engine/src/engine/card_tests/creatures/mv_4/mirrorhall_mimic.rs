//! `cards/creatures/mv_4/mirrorhall_mimic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghastly Mimicry: "At the beginning of your upkeep, create a token that's
/// a copy of enchanted creature, except it's a Spirit in addition to its
/// other types. If Ghastly Mimicry would be put into a graveyard from
/// anywhere, exile it instead." The Aura goes when its creature dies (CR
/// 704.5m), and the graveyard it would go to is replaced by exile, so it
/// cannot be disturbed a second time.
#[test]
fn ghastly_mimicry_copies_its_creature_each_upkeep_and_is_exiled_not_buried() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[mirrorhall_mimic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    disturb_the_mimic(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, mirrorhall_mimic()).expect("Ghastly Mimicry is out");
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(elves)
    );

    pass_until(&mut engine, |e| e.state().turn.active != p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    let copies: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).is_some_and(|o| o.card.is_none()))
        .collect();
    assert_eq!(copies.len(), 1, "one token at the upkeep");
    let subtypes = engine
        .state()
        .object(copies[0])
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        subtypes.contains(baylee_core::generated::subtypes::creature::ELF)
            && subtypes.contains(baylee_core::generated::subtypes::creature::SPIRIT),
        "a copy of the Elves that is a Spirit as well"
    );

    kill(&mut engine, elves);
    assert!(
        card_in(&engine, ZoneLocation::Graveyard(p0), mirrorhall_mimic()).is_none(),
        "the Aura never reached the graveyard"
    );
    assert!(
        card_in(&engine, ZoneLocation::Exile(p0), mirrorhall_mimic()).is_some(),
        "it was exiled instead"
    );
}

/// The same sentence on the stack: "from anywhere" includes a Ghastly
/// Mimicry that is countered.
#[test]
fn a_countered_ghastly_mimicry_is_exiled_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(111, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[mirrorhall_mimic()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    disturb_the_mimic(&mut engine, p0, elves);
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("Ghastly Mimicry is on the stack");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, counterspell());
    aim_at(&mut engine, p1, spell);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        card_in(&engine, ZoneLocation::Graveyard(p0), mirrorhall_mimic()).is_none(),
        "the countered Aura never reached the graveyard"
    );
    assert!(
        card_in(&engine, ZoneLocation::Exile(p0), mirrorhall_mimic()).is_some(),
        "it was exiled instead"
    );
}

/// The mana value of a back face (CR 202.3b). Ravager of the Fells, up on
/// the battlefield, has Huntmaster of the Fells's mana value, 4 (CR 712.8e);
/// Ghastly Mimicry, cast with disturb for {3}{U}{U}, has Mirrorhall Mimic's
/// 4 on the stack (CR 712.8c) and on the battlefield; and the token Ghastly
/// Mimicry makes at the upkeep is a copy of the Ravager's back face, whose
/// mana value is 0. Abrupt Decay ("nonland permanent with mana value 3 or
/// less") then offers the copy and neither card.
///
/// Before, a back face's mana value was its own mana cost's: the Ravager
/// was a 0 and on the Decay's menu, and Ghastly Mimicry a 5.
#[test]
#[allow(clippy::too_many_lines)] // one game, from the disturb to the Decay's menu
fn a_back_face_has_its_front_faces_mana_value_and_a_copy_of_one_has_none() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(112, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                huntmaster_of_the_fells(),
            ],
        )
        .hand(0, &[mirrorhall_mimic()])
        .battlefield(1, &[swamp(), forest()])
        .hand(1, &[abrupt_decay()])
        .start();
    let ravager =
        on_battlefield(&engine, p0, huntmaster_of_the_fells()).expect("Huntmaster is out");
    // Turned over by the harness before the first turn: what it did as it
    // turned is not what this test reads, so the face is switched without
    // the transform that would trigger it.
    let def = baylee_cards::by_index(huntmaster_of_the_fells()).expect("Huntmaster is in the pool");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .switch_face(ravager, def, 1);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(face_shown(&engine, ravager), 1, "Ravager of the Fells");
    let mana_value = |engine: &Engine<RegistryLookup>, id: ObjectId| {
        engine
            .state()
            .object(id)
            .map(|o| o.characteristics().mana_value())
    };
    assert_eq!(
        mana_value(&engine, ravager),
        Some(4),
        "Ravager of the Fells has Huntmaster's {{2}}{{R}}{{G}}"
    );

    disturb_the_mimic(&mut engine, p0, ravager);
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("Ghastly Mimicry is on the stack");
    assert_eq!(
        mana_value(&engine, spell),
        Some(4),
        "cast transformed, it has Mirrorhall Mimic's mana value, not the disturb cost's 5"
    );
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, mirrorhall_mimic()).expect("Ghastly Mimicry is out");
    assert_eq!(
        mana_value(&engine, aura),
        Some(4),
        "and so it has on the battlefield"
    );

    // To p0's next main phase: at its upkeep Ghastly Mimicry copies the
    // Ravager. Nobody cast two spells in a turn, so the Ravager stays.
    pass_until(&mut engine, |e| e.state().turn.active != p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(face_shown(&engine, ravager), 1, "still the Ravager");
    let copies: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).is_some_and(|o| o.card.is_none()))
        .collect();
    assert_eq!(copies.len(), 1, "one token at the upkeep");
    let copy = copies[0];
    assert_eq!(
        pt(&engine, copy),
        (4, 4),
        "a copy of the face that is up (CR 707.8)"
    );
    assert_eq!(
        mana_value(&engine, copy),
        Some(0),
        "a copy of a back face has mana value 0"
    );

    // p1 answers with Abrupt Decay in p0's main phase.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let decay = in_hand(&engine, p1, abrupt_decay()).expect("the Decay is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: decay })
        .expect("a Swamp and a Forest pay {B}{G}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Decay asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&copy),
        "the copy is a nonland permanent with mana value 0: {options:?}"
    );
    assert!(
        !options.contains(&ravager),
        "the Ravager is a four: {options:?}"
    );
    assert!(
        !options.contains(&aura),
        "Ghastly Mimicry is a four: {options:?}"
    );
    assert_eq!(options.len(), 1, "{options:?}");
}
