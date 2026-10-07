//! `cards/enchantments/mv_2/circle_of_protection_black.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Circle of Protection: Black: "{1}: The next time a black source of your
/// choice would deal damage to you this turn, prevent that damage."
/// Both the black Festering Goblin and a green Llanowar Elves attack
/// together: the choice may only name the Goblin, never the green Elves,
/// and only the Goblin's combat damage is prevented — the Elves' 1 still
/// connects, which is what proves the shield stops the one chosen source
/// and nothing else (an over-broad "prevent every attacker" bug would also
/// leave life at 20 here). The shield does not linger: once it has
/// prevented that one event, it is gone.
#[test]
fn circle_of_protection_black_prevents_a_chosen_black_attacker_and_only_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cop = circle_of_protection_black();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cop, forest()])
        .battlefield(1, &[festering_goblin(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let goblin = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    reach_their_main_phase(&mut engine, p1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&goblin) && attackers.contains(&elf),
        "both the Goblin and the Elves may attack: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(goblin, Defender::Player(p0)), (elf, Defender::Player(p0))],
            },
        )
        .expect("both attackers came out of the list that offered them");
    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    };
    assert!(blockers.is_empty(), "p0 has nothing to block with");
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, cop, 0);
    let selected = baylee_core::ids::DamageSourceRef {
        object: goblin,
        version: engine
            .state()
            .object(goblin)
            .expect("source exists")
            .version,
    };
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![selected],
        "a black source of your choice: the Goblin, never the green Elves"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .expect("off the list");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the Goblin's combat damage was prevented, but the Elves' 1 still \
         connected: the shield stopped the chosen source and nothing else"
    );
    assert!(
        engine.state().shields.is_empty(),
        "\"the next time\": once it has prevented one event, the shield is gone"
    );
}
