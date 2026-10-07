//! `cards/creatures/mv_2/alaborn_musketeer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alaborn Musketeer is `{1}{W}` for a 2/1 Human Soldier and one line of
/// text: reach. Reach is not a body, it is an answer to exactly one question
/// the combat step asks — so the test builds the board that asks it, casting
/// the Musketeer and then aiming a 4/4 flier at its controller on the next
/// turn. The block offer is the whole proof, and the untapped 1/1 standing
/// beside it is the control: same seat, same untapped state, and not offered
/// against anything that flies. Declaring the block then shows what reach is
/// for — the 2/1 dies to the flier it was allowed to block and the flier
/// lives on — so the block resolved and was not merely a menu entry.
#[test]
fn alaborn_musketeer_reaches_the_flier_the_elf_beside_it_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), quiet_creature()])
        .hand(0, &[alaborn_musketeer()])
        .battlefield(1, &[sphinx_of_the_final_word()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf is kept untapped on purpose: it is the control that must be
    // able to block in every way but reach, and a creature tapped for mana
    // is refused as a blocker for a reason that has nothing to do with the
    // keyword. Two Plains pay the `{1}{W}` on their own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, alaborn_musketeer());
    pass_until(&mut engine, stack_is_empty);

    let musketeer =
        on_battlefield(&engine, p0, alaborn_musketeer()).expect("the Musketeer resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is still out");
    assert_eq!(pt(&engine, musketeer), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, musketeer).contains(KeywordSet::REACH),
        "reach, read off the layer system rather than off the card file"
    );

    // Across the table, and the flier's flying is the premise of everything
    // below: without it the question this test is about is never asked.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let flier = on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the Sphinx is out");
    assert!(
        keywords(&engine, flier).contains(KeywordSet::FLYING),
        "the attacker has to fly, or the block menu below proves nothing"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(attackers.contains(&flier), "{attackers:?}");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(flier, Defender::Player(p0))],
            },
        )
        .unwrap();

    // The blocker menu is not the next thing the engine says: CR 508.2 hands
    // priority round once the attack is declared, so the walk has to answer
    // that before the question this test is about is asked.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!(
            "the Musketeer can block, so p0 is asked, got {:?}",
            engine.pending()
        )
    };
    let reachable = blockers
        .iter()
        .find(|option| option.blocker == musketeer)
        .expect("reach is what puts the Musketeer on the blocker menu");
    assert!(
        reachable.attackers.contains(&flier),
        "and points it at the flier: {:?}",
        reachable.attackers
    );
    assert!(
        !blockers
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&flier)),
        "the untapped 1/1 beside it has no reach and is no answer to a \
         flier: {blockers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(musketeer, flier)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, alaborn_musketeer()).is_some()
    });
    assert!(
        in_graveyard(&engine, p0, alaborn_musketeer()).is_some(),
        "the 2/1 spent its life on the block (CR 510.1a: four damage on one \
         toughness), which is what tells a resolved block from an offer"
    );
    assert!(
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).is_some(),
        "and the 4/4 survives the two the block dealt back to it"
    );
}
