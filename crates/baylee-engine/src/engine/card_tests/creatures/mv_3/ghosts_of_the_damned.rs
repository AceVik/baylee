//! `cards/creatures/mv_3/ghosts_of_the_damned.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghosts of the Damned is a printed 0/2 Spirit whose whole text is "{T}:
/// Target creature gets -1/-0 until end of turn."
///
/// The scenario crosses a turn boundary before pressing it, because a creature
/// cast this turn may not pay a tap symbol (CR 302.6) — so the permanent that
/// is tapped is the one the cast actually left behind and not a body the
/// harness seated. Four creatures stand on the table (two of mine, one across
/// it, and the Ghosts itself), which is what makes "target creature" readable
/// as any creature rather than "creatures you control". The numbers the card
/// prints then separate the readings: the aimed-at Elf is a (0, 1) — one power
/// gone and the toughness the card never touches — and the three creatures
/// nobody named stay printed 1/1s and 0/2.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ghosts_of_the_damned_taps_to_take_one_power_off_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[ghosts_of_the_damned()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{B}{B} off the three Swamps, with both Elves named as kept back: they
    // are two of the creatures this test reads back afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, three black, and no Elf tapped for any of it"
    );
    cast_with_floating(&mut engine, p0, ghosts_of_the_damned());
    pass_until(&mut engine, stack_is_empty);
    let ghosts = on_battlefield(&engine, p0, ghosts_of_the_damned())
        .expect("the Ghosts resolved onto the table");
    assert_eq!(pt(&engine, ghosts), (0, 2), "the body the card prints");

    // Summoning sickness (CR 302.6): the Ghosts arrived this turn, so its
    // {T} is not even offered yet. One turn has to pass and come back.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, ghosts),
        "and the untap step stood the Ghosts back up"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves of mine, one of which stays alone"
    );
    let (target, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "a printed 1/1 before the shrink"
    );

    activate(&mut engine, p0, ghosts_of_the_damned(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&target) && options.contains(&bystander),
        "both creatures you control are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"creatures you control\": the Elf across \
         the table is offered, and this test declines it: {options:?}"
    );
    assert!(
        options.contains(&ghosts),
        "and the Ghosts is a creature itself, so its own body may be aimed at: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "four creatures stand on the table and nothing else is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf the question offered is a legal answer");
    assert!(
        is_tapped(&engine, ghosts),
        "{{T}} is the whole price, and CR 601.2h pays it after the target"
    );
    assert!(
        !stack_is_empty(&engine),
        "shrinking a creature is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (0, 1),
        "-1/-0 on a printed 1/1: one power gone, and the toughness the card \
         never touches"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody aimed at is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "an Elf across the table that the ability was not pointed at would \
         read (0, 1) here if the pump had leaked"
    );
    assert_eq!(
        pt(&engine, ghosts),
        (0, 2),
        "the Ghosts aimed at the Elf, so its own body never moved"
    );

    // "until end of turn" is the other half of the sentence: past p0's
    // cleanup the same Elf has its printed power back.
    let mut reached_p1 = false;
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == p1
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1)
        {
            reached_p1 = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand.into_iter().take(usize::from(count)).collect(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected on the way past the end of the turn: {other:?}"),
        }
    }
    assert!(reached_p1, "p1 takes its own turn");
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "\"until end of turn\": across the boundary the Elf is a printed 1/1 again"
    );
    assert!(
        on_battlefield(&engine, p0, ghosts_of_the_damned()).is_some(),
        "and the Ghosts, which the effect never touched, is still on the table"
    );
}
