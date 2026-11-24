//! `cards/sorceries/mv_2/do_or_die.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn do_or_die() -> CardIndex {
    card_index("f25af42d-f1bf-4bd3-aded-ecadafdbf6e6")
}

/// Do or Die lets the caster separate only the targeted player's creatures;
/// that player chooses which pile is destroyed. The whole chosen pile dies,
/// even through a regeneration shield; the other pile and the caster's own
/// creatures survive.
#[test]
fn do_or_die_destroys_the_target_players_whole_chosen_pile_without_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .hand(0, &[do_or_die()])
        .battlefield(
            1,
            &[swamp(), lotleth_troll(), llanowar_elves(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Buy the shield before casting: an empty stack lets the regeneration
    // ability resolve without also advancing the spell under test.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let troll = on_battlefield(&engine, p1, lotleth_troll()).expect("the Troll is seated");
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 1,
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().object(troll).unwrap().regeneration_shields,
        1,
        "the regeneration ability resolved and the spell is still uncast"
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, do_or_die());
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("the spell asks for its target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![p0, p1], "any player may be targeted");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "both opposing Elves are seated");
    let own_elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("the caster's Elf is seated");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert_eq!(
        player, p0,
        "the caster separates the targeted player's creatures"
    );
    assert_eq!((min, max), (0, 3), "either pile may be empty");
    assert_eq!(prompt, ChoicePrompt::FirstPile);
    assert_eq!(options, vec![troll, elves[0], elves[1]]);
    assert!(
        !options.contains(&own_elf),
        "the caster's creatures are not among the targeted player's piles"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![troll, elves[0]],
            },
        )
        .unwrap();
    let Pending::ChoosePile { player, piles, .. } = engine.pending().clone() else {
        panic!("expected the targeted player's pile choice")
    };
    assert_eq!(
        player, p1,
        "the targeted player chooses the pile to destroy"
    );
    assert_eq!(piles, vec![vec![troll, elves[0]], vec![elves[1]]]);
    engine.apply(p1, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, lotleth_troll()).is_some());
    assert_eq!(
        engine.state().object(elves[0]).map(|o| o.zone),
        Some(Zone::Graveyard),
        "destroying a pile destroys every creature in it, not only the first"
    );
    assert_eq!(
        engine.state().object(elves[1]).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the unchosen pile survives"
    );
    assert_eq!(
        engine.state().object(own_elf).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the caster's creature was never part of either pile"
    );
}
