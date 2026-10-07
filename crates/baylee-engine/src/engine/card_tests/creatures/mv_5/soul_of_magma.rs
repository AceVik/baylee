//! `cards/creatures/mv_5/soul_of_magma.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1566af29-bd7b-41bb-a589-499348fbd32c"

/// Soul of Magma — {3}{R}{R} Creature — Spirit 2/2: "Whenever you cast a
/// Spirit or Arcane spell, this creature deals 1 damage to target creature."
///
/// Two copies stand on the board because the sentence is per-creature and its
/// number is the whole card: one Kodama of the North Tree — a Spirit whose own
/// text is nothing but trample and shroud, so casting it puts no trigger of
/// its own beside the Soul of Magma's — raises one target question per copy,
/// and the two questions are aimed one at a printed 1/1 and one at a printed
/// 2/2. The 1/1 dies (CR 704.5g) and the 2/2 is still standing afterwards,
/// which is the only pair of readings that fixes the damage at exactly one —
/// zero would leave the Elf alive and two would take the Knight with it. A
/// Llanowar Elves cast last is the control for the filter: it is a creature
/// spell and neither a Spirit nor an Arcane one, so the same board must not
/// ask for a target over it at all.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn soul_of_magma_shoots_one_creature_for_each_copy_that_watches_a_spirit_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                soul_of_magma(),
                soul_of_magma(),
            ],
        )
        .hand(0, &[kodama_of_the_north_tree(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), benalish_knight()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let knight = on_battlefield(&engine, p1, benalish_knight()).expect("their Knight is out");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1, which one damage puts into the graveyard (CR 704.5g)"
    );
    assert_eq!(
        pt(&engine, knight),
        (2, 2),
        "a printed 2/2, which one damage leaves standing"
    );

    // Six Forests are the whole of p0's mana: they pay the Spirit spell's
    // {2}{G}{G}{G} and leave one green floating for the control cast at the
    // end, since the whole scenario lives in this one main phase (CR 500.5).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, and neither Soul of Magma makes mana of its own"
    );
    cast_with_floating(&mut engine, p0, kodama_of_the_north_tree());

    // Each copy watches the cast on its own, so one Spirit spell raises one
    // target question per Soul of Magma — the second question below is what
    // says the ability belongs to the creature and not to the cast.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, elf);
    assert!(
        options.contains(&elf) && options.contains(&knight),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, knight);
    assert!(
        options.contains(&elf) && options.contains(&knight),
        "the second copy asks the same question of the same board: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    // The control: Llanowar Elves is a creature spell with neither of the two
    // subtypes the trigger reads, so the walk must end on an empty stack and
    // never on a question. `pass_until` would stop on a `ChooseTargets` with a
    // panic, so the count is taken by hand to say what the failure would be.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    let mut asked = 0usize;
    let mut steps = 0usize;
    loop {
        steps += 1;
        assert!(steps <= 20, "the control spell never resolved");
        match engine.pending().clone() {
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                asked += 1;
                break;
            }
            other => panic!("unexpected while the control spell resolved: {other:?}"),
        }
    }
    assert_eq!(
        asked, 0,
        "an Elf is a creature spell and neither a Spirit nor an Arcane one, so \
         no Soul of Magma asks for a target over it"
    );

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, benalish_knight()).is_some(),
        "and one damage on a printed 2/2 is not — zero would have left the Elf \
         standing and two would have taken the Knight with it, so the shot is \
         exactly the one the card prints"
    );
    assert!(
        on_battlefield(&engine, p0, soul_of_magma()).is_some(),
        "the Soul of Magma that did the shooting outlives the creature it shot"
    );
    assert!(
        on_battlefield(&engine, p0, kodama_of_the_north_tree()).is_some(),
        "the Spirit spell that set the triggers off resolved onto the table"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the control spell resolved too, so the empty question count above \
         is a resolved cast and not a stuck one"
    );
}
