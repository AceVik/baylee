//! `cards/creatures/mv_4/jeska_warrior_adept.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jeska, Warrior Adept — {2}{R}{R}, a legendary 3/1 Human Barbarian Warrior
/// with first strike and haste. Beyond the two keywords her whole printed text
/// is "{T}: Jeska deals 1 damage to any target", so the scenario plays exactly
/// that: she is seated already on the battlefield (so the tap symbol is a
/// payable price and not summoning sickness), the board carries no land at all,
/// and the target question is read while it stands — the Elf across the table
/// *and* both seats, which is what "any target" (CR 115.4) means. The single
/// point then kills a printed 1/1 while both life totals stay where they were.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn jeska_warrior_adept_taps_for_one_damage_at_any_target() {
    fn jeska_warrior_adept() -> CardIndex {
        card_index("3186fddd-23fd-440c-ad61-b4130e00f765")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jeska_warrior_adept()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let jeska = on_battlefield(&engine, p0, jeska_warrior_adept()).expect("Jeska is on the table");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf is across the table");
    assert_eq!(pt(&engine, jeska), (3, 1), "the body the card prints");
    let printed = keywords(&engine, jeska);
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "First strike reaches the permanent: {printed:?}"
    );
    assert!(
        printed.contains(KeywordSet::HASTE),
        "and haste with it: {printed:?}"
    );

    // The whole price of the ability is the tap symbol, and there is no land on
    // this board and nothing floating: `can_afford` reads the pool, so an offer
    // read here cannot have been bought with mana.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no source on this board, and nothing floating"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(jeska, 0)),
        "an untapped Jeska is a paid {{T}}, so the one line she prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, jeska_warrior_adept(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // tap symbol has not been paid while this question stands.
    assert!(
        !is_tapped(&engine, jeska),
        "the {{T}} is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert!(is_tapped(&engine, jeska), "{{T}} was the whole price");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player \
         whose board it stood on"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and Jeska's controller gained nothing for it"
    );
    assert!(
        on_battlefield(&engine, p0, jeska_warrior_adept()).is_some(),
        "the cost was the tap, so Jeska is still on the battlefield"
    );
}
