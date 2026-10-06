//! `cards/creatures/mv_6/zephid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// Angel of Mercy — {4}{W} 3/3 flying. The bystander every claim here needs:
// it is a creature on the table with neither evasion of its own that matters
// to a Plowshares, so a target menu that offers it and withholds Zephid is
// reading shroud and not a filter that had merely narrowed.

/// Zephid prints two lines and both are keywords: "Flying" and "Shroud (This
/// creature can't be the target of spells or abilities.)". The printed 3/4
/// body is read off the projection and the flying is read off the same
/// characteristics, but shroud is the half a card file cannot demonstrate: it
/// is a *prohibition*, so it has to be played against a spell that wants to
/// name it. Swords to Plowshares is that spell and the Angel of Mercy is the
/// control — the same Plowshares offers the Angel and must not offer Zephid,
/// so an offer holding exactly one of them is the difference and not a board
/// that had nothing to aim at. The exiled permanent afterwards is the other
/// half: shroud is not hexproof, and it declines *both* seats' spells.
#[test]
#[allow(clippy::too_many_lines)]
fn zephid_flies_and_its_shroud_declines_its_own_controllers_swords() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(401, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[zephid(), swords_to_plowshares()])
        .battlefield(1, &[angel_of_mercy()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let angel = on_battlefield(&engine, p1, angel_of_mercy()).expect("the Angel is out");
    assert_eq!(pt(&engine, angel).1, 3, "a 3/3, so it is the control");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");

    // {4}{U}{U} off the four Islands and two of the Plains, with the Elves kept
    // back: it is a creature this test reads afterwards, and the {W} that
    // follows has to come out of a land that is still standing.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, zephid());
    pass_until(&mut engine, |e| on_battlefield(e, p0, zephid()).is_some());
    let the_zephid = on_battlefield(&engine, p0, zephid()).expect("the Zephid resolved");
    assert_eq!(pt(&engine, the_zephid), (3, 4), "the body the card prints");
    let granted = keywords(&engine, the_zephid);
    assert!(granted.contains(KeywordSet::FLYING), "Flying");
    assert!(
        granted.contains(KeywordSet::SHROUD),
        "Shroud, and on the battlefield rather than in the card file"
    );

    // A Plowshares of my own, aimed at my own board. The Elf is there so an
    // empty exclusion is not an empty board, and the Angel across the table is
    // the control: a menu with creatures on both sides of it and Zephid on
    // neither is the printed sentence and nothing else.
    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf) && options.contains(&angel),
        "the Elves and the Angel — the two creatures on the board with no \
         shroud — are both offered: {options:?}"
    );
    assert!(
        !options.contains(&the_zephid),
        "and the Zephid is not, even to its own controller's spell: \
         {options:?}"
    );
    assert!(
        matches!(
            engine.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![the_zephid],
                },
            ),
            Err(EngineError::IllegalAction(_))
        ),
        "naming it anyway is refused, because the offer is what the engine \
         validates against"
    );
    assert!(
        on_battlefield(&engine, p0, zephid()).is_some(),
        "and the refusal cost it nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .expect("the Angel was one of the options the question enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, angel_of_mercy()).is_none(),
        "the spell resolved against the creature it was allowed to name"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == angel_of_mercy()))),
        "and exile is where Plowshares puts it"
    );
    let mine = in_hand(&engine, p0, zephid());
    assert!(mine.is_none(), "the Zephid was never in my hand");
    assert!(
        on_battlefield(&engine, p0, zephid()).is_some()
            && engine
                .state()
                .object(the_zephid)
                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield),
        "the shroud held: the creature is still the printed 3/4 it was cast as"
    );
    assert_eq!(
        pt(&engine, the_zephid),
        (3, 4),
        "with both keywords still on it after the turn's spells were done"
    );
}
