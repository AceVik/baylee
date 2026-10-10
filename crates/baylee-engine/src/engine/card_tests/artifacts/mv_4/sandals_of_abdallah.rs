//! `cards/artifacts/mv_4/sandals_of_abdallah.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandals of Abdallah — {4} artifact: "{2}, {T}: Target creature gains
/// islandwalk until end of turn. When that creature dies this turn, destroy
/// this artifact."
fn sandals_of_abdallah() -> CardIndex {
    card_index("ff77074f-48ef-4c01-8ede-4e9be3e483f4")
}

/// Turn one, main phase, with Sandals out and three `land`s, a Gray Ogre
/// (the creature to pump) and a Llanowar Elves (some other creature) on my
/// side, and `spell` in hand. Everything is tapped for mana, the Sandals
/// are activated at the Ogre (`{2}` paid from the pool, one mana left for
/// `spell`) and resolved.
fn pumped(land: CardIndex, spell: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                sandals_of_abdallah(),
                land,
                land,
                land,
                gray_ogre(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sandals = on_battlefield(&engine, p0, sandals_of_abdallah()).expect("Sandals are out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre is out");
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    activate(&mut engine, p0, sandals_of_abdallah(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, sandals), "{{T}} was paid");
    assert!(keywords(&engine, ogre).contains(KeywordSet::ISLANDWALK));
    (engine, sandals, ogre)
}

/// Casts `spell` off the floating mana at `target` and resolves everything.
#[track_caller]
fn cast_at_and_resolve(engine: &mut Engine<RegistryLookup>, spell: CardIndex, target: ObjectId) {
    let p0 = PlayerId::new(0);
    cast_with_floating(engine, p0, spell);
    aim_at(engine, p0, target);
    pass_until(engine, stack_is_empty);
}

/// Islandwalk is real: against an Island the Ogre is not offered a blocker,
/// and without the Sandals' pump (the control) it is.
#[test]
fn sandals_of_abdallah_grants_islandwalk_that_makes_the_target_unblockable_by_island_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pump in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[sandals_of_abdallah(), mountain(), mountain(), gray_ogre()],
            )
            .battlefield(1, &[island(), gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, p1);
        reach_their_main_phase(&mut engine, p0);
        let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
        let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
        assert!(!keywords(&engine, ogre).contains(KeywordSet::ISLANDWALK));
        if pump {
            tap_all_mana(&mut engine, p0);
            activate(&mut engine, p0, sandals_of_abdallah(), 0);
            let options = aim_at(&mut engine, p0, ogre);
            assert!(options.contains(&ogre), "any creature is a target");
            assert!(options.contains(&theirs), "either side's: {options:?}");
            pass_until(&mut engine, stack_is_empty);
            assert!(keywords(&engine, ogre).contains(KeywordSet::ISLANDWALK));
        }
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(ogre, Defender::Player(p1))],
                },
            )
            .expect("my Ogre may attack on turn three of the table");
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
            unreachable!("the predicate just matched")
        };
        let can_block = blockers
            .iter()
            .any(|b| b.blocker == theirs && b.attackers.contains(&ogre));
        assert_eq!(can_block, !pump, "islandwalk (pump = {pump}) vs an Island");
    }
}

/// The grant lasts until end of turn and no longer.
#[test]
fn sandals_of_abdallah_islandwalk_ends_with_the_turn() {
    let p1 = PlayerId::new(1);
    let (mut engine, _sandals, ogre) = pumped(mountain(), lightning_bolt());
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, ogre).contains(KeywordSet::ISLANDWALK));
}

/// The pumped creature dies this turn: the delayed trigger goes on the stack
/// (the Sandals are still there until it resolves) and then destroys them.
#[test]
fn sandals_of_abdallah_is_destroyed_when_the_pumped_creature_dies_this_turn() {
    let p0 = PlayerId::new(0);
    let (mut engine, sandals, ogre) = pumped(mountain(), lightning_bolt());
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, |e| in_graveyard(e, p0, gray_ogre()).is_some());
    assert!(!stack_is_empty(&engine), "the delayed trigger is waiting");
    assert!(
        on_battlefield(&engine, p0, sandals_of_abdallah()).is_some(),
        "destroyed only when the trigger resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sandals_of_abdallah()).is_none());
    assert!(in_graveyard(&engine, p0, sandals_of_abdallah()).is_some());
    let _ = sandals;
}

/// Dying on a later turn is not "this turn": the Sandals stay.
#[test]
fn sandals_of_abdallah_stay_when_the_creature_dies_on_a_later_turn() {
    let p1 = PlayerId::new(1);
    let (mut engine, _sandals, ogre) = pumped(mountain(), lightning_bolt());
    reach_their_main_phase(&mut engine, p1);
    kill(&mut engine, ogre);
    assert!(in_graveyard(&engine, PlayerId::new(0), gray_ogre()).is_some());
    assert!(on_battlefield(&engine, PlayerId::new(0), sandals_of_abdallah()).is_some());
}

/// Leaving by another way (bounced, exiled) is not dying: the Sandals stay.
#[test]
fn sandals_of_abdallah_stay_when_the_creature_leaves_without_dying() {
    let p0 = PlayerId::new(0);
    // Unsummon: back to hand.
    let (mut engine, _s, ogre) = pumped(island(), unsummon());
    cast_at_and_resolve(&mut engine, unsummon(), ogre);
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_hand(&engine, p0, gray_ogre()).is_some(), "bounced");
    assert!(on_battlefield(&engine, p0, sandals_of_abdallah()).is_some());

    // Swords to Plowshares: exile.
    let (mut engine, _s, ogre) = pumped(plains(), swords_to_plowshares());
    cast_at_and_resolve(&mut engine, swords_to_plowshares(), ogre);
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_none(), "exiled");
    assert!(on_battlefield(&engine, p0, sandals_of_abdallah()).is_some());
}

/// A different creature dying this turn is not the pumped one: the Sandals
/// stay and the pumped Ogre lives.
#[test]
fn sandals_of_abdallah_stay_when_a_different_creature_dies() {
    let p0 = PlayerId::new(0);
    let (mut engine, _s, ogre) = pumped(mountain(), lightning_bolt());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf");
    cast_at_and_resolve(&mut engine, lightning_bolt(), elf);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, sandals_of_abdallah()).is_some());
    let _ = ogre;
}
