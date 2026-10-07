//! `cards/lands/deserts/ifnir_deadlands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ifnir Deadlands is a Desert land: `{T}: Add {C}`, `{T}, pay 1 life: Add
/// {B}`, and `{2}{B}{B}, {T}, Sacrifice a Desert: Put two -1/-1 counters on
/// target creature an opponent controls. Activate only as a sorcery.` The
/// third line is played whole, and every noun in it is held down by something
/// else on the board: four Swamps pay the `{2}{B}{B}` while being no Deserts,
/// the Elf beside the Deadlands is a permanent of mine that is neither a
/// Desert nor a creature of theirs, and the Wurm across the table is the only
/// legal target. The Deadlands pays the sacrifice with itself — a Desert sits
/// on its own menu, as Krark-Clan Ironworks does — so the card leaves the
/// battlefield and the counters read back off the body they were aimed at.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn ifnir_deadlands_eats_itself_for_two_minus_counters_on_the_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                ifnir_deadlands(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[rootbreaker_wurm()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let deadlands = on_battlefield(&engine, p0, ifnir_deadlands()).expect("the Deadlands is out");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let printed = pt(&engine, wurm);

    // A cost is read off the mana *pool* and not off the lands that could
    // still be tapped: nothing floats, so the third line is not offered at
    // all, and the offer asserted further down is only worth reading because
    // this one was made first.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the active seat holds a quiet main phase: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(deadlands, 2)),
        "nothing floats, so {{2}}{{B}}{{B}} is not payable: {:?}",
        legal.abilities
    );

    // Four Swamps, and the Deadlands kept back by name: its own {{T}} is half
    // the printed cost, and `tap_all_mana` would have spent it on a
    // colourless.
    tap_all_mana_but(&mut engine, p0, Some(ifnir_deadlands()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4,
        "the Swamps pay in black"
    );
    assert!(
        !is_tapped(&engine, deadlands),
        "the source is still standing"
    );

    // Ability 2: 0 is {T}: Add {C} and 1 is {T}, pay 1 life: Add {B}.
    activate(&mut engine, p0, ifnir_deadlands(), 2);

    // One activation, two questions, answered in the order they arrive
    // rather than the order they are expected: the target is chosen at
    // CR 601.2c and the sacrifice is paid at CR 601.2h.
    let mut aimed = false;
    let mut paid = false;
    for _ in 0..8 {
        if aimed && paid {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(
                    options,
                    vec![wurm],
                    "\"target creature an opponent controls\" — my own Elf is a \
                     creature I control and is not on it: {options:?}"
                );
                aimed = true;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![wurm],
                        },
                    )
                    .expect("the Wurm is one of the options it just enumerated");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Desert, no more and no fewer");
                assert!(
                    options.contains(&deadlands),
                    "the Deadlands is a Desert, so it is on its own menu: {options:?}"
                );
                assert!(
                    !options.contains(&elf),
                    "the Elf is a creature and no Desert: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    1,
                    "and the four Swamps under it are no Deserts either: {options:?}"
                );
                paid = true;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![deadlands],
                        },
                    )
                    .expect("the Desert that pays is the card itself");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the third line resolves: {other:?}"),
        }
    }
    assert!(aimed, "the ability is not activated without a target");
    assert!(paid, "and not paid without a Desert to eat");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, ifnir_deadlands()).is_none(),
        "the Deadlands paid the sacrifice with itself"
    );
    assert!(
        in_graveyard(&engine, p0, ifnir_deadlands()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, wurm),
        (printed.0 - 2, printed.1 - 2),
        "two -1/-1 counters on a {printed:?} body: one counter would leave it \
         a point bigger on both stats"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and the counters went to the target and nowhere else"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the {{T}}, pay 1 life line was never pressed, so the black came off \
         the Swamps"
    );
}
