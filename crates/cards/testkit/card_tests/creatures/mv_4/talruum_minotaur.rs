//! `cards/creatures/mv_4/talruum_minotaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c5661500-c48f-4f6d-bbe7-c8bd7118d862"

/// Talruum Minotaur is `{2}{R}{R}` for a 3/3 whose whole printed text is
/// Haste, and Haste is a keyword no reading of the card file demonstrates: it
/// only ever matters in the combat step of the turn the creature arrived. So
/// the Minotaur is cast and then has to be *offered* by the very same turn's
/// declare-attackers question — with a Llanowar Elves cast beside it in the
/// same main phase as the control, because a creature that entered this turn
/// is otherwise withheld, which is exactly the rule the keyword buys its way
/// out of. The attack is followed through to the end step so the haste reads
/// as damage and not merely as an offer.
#[test]
fn talruum_minotaur_attacks_the_turn_it_is_cast_while_a_fresh_elf_may_not() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), forest()],
        )
        .hand(0, &[talruum_minotaur(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Mountains pay the Minotaur and the Forest is kept back for the Elf
    // below, so neither spell can be paid out of the other's colour: the
    // Minotaur's `{R}{R}` has no green source and the Elf's `{G}` is not
    // something a Mountain could cover.
    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    tap_mana_where(&mut engine, p0, |id| id != forest_land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains tapped and the Forest left standing"
    );
    cast_with_floating(&mut engine, p0, talruum_minotaur());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let minotaur = on_battlefield(&engine, p0, talruum_minotaur()).expect("the Minotaur resolved");
    assert_eq!(pt(&engine, minotaur), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, minotaur).contains(KeywordSet::HASTE),
        "the printed Haste reaches the permanent"
    );

    // The control, cast in the same main phase so that both creatures entered
    // this turn and only the printed keyword can tell the two apart.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest is the only untapped source left, and it makes one green"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf resolved");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&minotaur),
        "a creature with haste may attack on the turn it entered: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elf),
        "and a creature without it may not, however untapped it stands: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(minotaur, Defender::Player(p1))],
            },
        )
        .expect("the Minotaur came out of the list that offered it");
    // The end step is past the combat damage step (CR 510.2); an empty stack
    // is already reached before damage is dealt, so that would prove nothing.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the 3/3 connected for three on the turn it was cast"
    );
}
