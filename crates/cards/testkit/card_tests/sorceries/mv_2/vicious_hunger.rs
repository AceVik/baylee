//! `cards/sorceries/mv_2/vicious_hunger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vicious Hunger is `{B}{B}` for "Vicious Hunger deals 2 damage to target
/// creature and you gain 2 life." The board is built so that neither half can
/// be borrowed from anything else: the creature it is aimed at is a printed
/// 1/1, which is exactly what two damage is lethal to, and the other two
/// creatures on the table are the controls that say the spell is *aimed*
/// rather than applied to the board — the second Elf across the table and the
/// one under the caster's own control are both still printed 1/1s afterwards.
/// Nothing on the table can move a life total, so the one point of life the
/// caster is up is the printed clause and not a land or a mana creature's.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vicious_hunger_kills_a_printed_one_one_and_gains_exactly_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[vicious_hunger()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my own Elf is out");
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves across the table, one of which dies"
    );
    let (prey, spare) = (elves[0], elves[1]);
    assert_eq!(
        pt(&engine, prey),
        (1, 1),
        "a printed 1/1 is exactly what two damage is lethal to"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and so is the bystander on this side of the table"
    );

    // Both Swamps and the Elf feed the cost, which is why the green is counted
    // below rather than assumed away: a mana creature is a mana route too
    // (#159), and `tap_all_mana` presses it.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2, "two Swamps, two black");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the Elf's own {{G}}, which no black cost can spend"
    );

    cast_with_floating(&mut engine, p0, vicious_hunger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "\"target creature\" is exactly one target"
    );
    assert!(
        options.contains(&prey) && options.contains(&spare) && options.contains(&mine),
        "\"target creature\" reaches every creature on the table, both sides \
         included: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "and names no player — the damage goes to a creature: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![prey],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        on_battlefield(&engine, p1, llanowar_elves()),
        Some(spare),
        "and the damage is aimed: the Elf nobody named never moved"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"target creature\" is not \"the table\": the caster's own Elf is untouched"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"you gain 2 life\" — two, and not a point per creature or per damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was targeted and never to its controller"
    );
    assert!(
        in_graveyard(&engine, p0, vicious_hunger()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "the {{B}}{{B}} came out of the pool"
    );
    assert_eq!(
        pool.total(),
        1,
        "and what is left is the Elf's green, which could not have paid it"
    );
}
