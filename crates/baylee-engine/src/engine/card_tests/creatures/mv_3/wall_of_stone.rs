//! `cards/creatures/mv_3/wall_of_stone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Stone — {1}{R}{R} — Creature — Wall, 0/8, whose entire printed
/// text is the defender keyword. The body and the keyword are one thing to
/// read off the card file but two things to play: the 0/8 has to arrive, so
/// the card is cast out of three tapped Mountains, and the defender has to
/// keep it out of the attack declaration *while it is untapped and past
/// summoning sickness* — which is why the board is walked across the
/// opponent's turn and back. A turn-one read would be satisfied by CR 302.6
/// alone, and the untapped Elf under the same seat is the control that says
/// the combat step really came: an offer without the Wall is defender and
/// not a phase the walk never reached. Both printed numbers are asserted
/// with it, because a defender that had lost its body would still be
/// correctly absent from the attackers.
#[test]
fn wall_of_stone_lands_as_a_defender_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[wall_of_stone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Mountains pay {1}{R}{R}; the Elf is named as the printing kept
    // back, because it is the control attacker below and a source tapped for
    // mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains tapped, and the Elf still standing"
    );
    cast_with_floating(&mut engine, p0, wall_of_stone());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_stone()).expect("the Wall resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, wall), (0, 8), "the body the card prints");
    assert!(
        types(&engine, wall).contains(TypeSet::CREATURE),
        "and it is the creature it prints"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "its whole printed text is the defender keyword"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{R}}{{R}} came out of the pool"
    );

    // Across the opponent's turn and back, so that the Wall is untapped and
    // no longer summoning sick: from here only the keyword can keep it out of
    // the attack declaration.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, wall),
        "the untap step stood the Wall up, so its absence below is not a tap"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "a creature with defender may not attack (CR 702.3b), however large its \
         toughness: {attackers:?}"
    );
}
