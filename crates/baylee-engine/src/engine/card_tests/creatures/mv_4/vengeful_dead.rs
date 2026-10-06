//! `cards/creatures/mv_4/vengeful_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vengeful Dead — {3}{B}, a 3/2 Zombie: "Whenever this creature or another
/// Zombie dies, each opponent loses 1 life."
///
/// Three deaths on one board read every word of that sentence. Killing this
/// seat's own Llanowar Elves is the control: a creature that is neither
/// Vengeful Dead nor a Zombie must leave both life totals alone, so the drain
/// below cannot be "something died". The Festering Goblin is the Zombie half of
/// the disjunction — a *different* printing, whose subtype is the whole of what
/// the filter looks at, and whose own dies trigger is answered on the way past
/// — and Vengeful Dead's own death is the "this creature" half. "Each
/// opponent" is read on both seats: p1 pays once per qualifying death and p0,
/// who owns the trigger, pays for none of them.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn vengeful_dead_drains_the_opponent_for_a_zombie_and_for_itself_but_not_for_an_elf() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[vengeful_dead()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {3}{B} off the four Swamps. Both Elves are named as the printings kept
    // back: the one under p0 is the control this test kills in a moment, and a
    // mana creature tapped for the cost would make the pool a count of
    // something other than the four Swamps.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps and no Elf: four black"
    );
    cast_with_floating(&mut engine, p0, vengeful_dead());
    pass_until(&mut engine, stack_is_empty);
    let dead = on_battlefield(&engine, p0, vengeful_dead()).expect("Vengeful Dead resolved");
    let zombie = on_battlefield(&engine, p0, festering_goblin()).expect("the Zombie is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    assert_eq!(pt(&engine, dead), (3, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing has died yet, so nothing has been lost"
    );

    // (1) A creature that is neither Vengeful Dead nor a Zombie.
    kill(&mut engine, elf);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Elves really died, so the silence below is the filter's and not a \
         death that never happened"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "an Elf dying is no Zombie dying and no Vengeful Dead dying: the \
         trigger is silent"
    );

    // (2) A Zombie that is not Vengeful Dead. The Goblin prints a dies trigger
    // of its own, so whatever question that raises is answered here, in the
    // order it arrives rather than in the order it is expected.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, zombie);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(
                    player, p0,
                    "the Zombie's controller answers its own trigger"
                );
                assert_eq!((min, max), (1, 1), "one target, and the trigger asks once");
                assert!(
                    options.contains(&dead),
                    "\"target creature\": Vengeful Dead is a creature and one of \
                     the options: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![dead],
                        },
                    )
                    .expect("the target the question offered is a legal answer");
            }
            Pending::Arrange {
                player,
                cards,
                piles,
                ..
            } => {
                let piles = crate::choice::default_arrangement(&cards, &piles)
                    .expect("an arrangement the engine asks can be answered");
                engine
                    .apply(player, PlayerAction::Arrange { piles })
                    .expect("the order the question offered is a legal answer");
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            other => panic!("unexpected while the dies triggers resolve: {other:?}"),
        }
    }

    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "the Zombie the state-based action destroyed is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"another Zombie dies\": a Zombie that is not Vengeful Dead drains the \
         opponent — where the Elf above drained nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"each opponent\" is not \"each player\": the seat that owns the \
         trigger pays nothing"
    );

    // (3) Vengeful Dead's own death, the `This` half of the same filter.
    kill(&mut engine, dead);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"this creature … dies\": the Zombie that carries the trigger drains \
         for itself too, one more life and not two"
    );
    assert!(
        in_graveyard(&engine, p0, vengeful_dead()).is_some(),
        "and it is in its owner's graveyard, so the second drain was its own \
         death and not another Zombie's"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "two deaths that count, and the trigger's owner still pays for neither"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf across the table never moved: no drain touched its owner's board"
    );
}
