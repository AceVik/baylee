//! `cards/enchantments/mv_4/reckless_assault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reckless Assault is an enchantment printing one line — "{1}, Pay 2 life:
/// This enchantment deals 1 damage to any target" — and neither half of that
/// price is anything a board can read off the card. Two Forests fill a pool
/// the `{1}` empties, the two life leave the controller, and the damage lands
/// on the Llanowar Elves opposite: one point on a printed 1/1 is lethal
/// (CR 704.5g), so the creature dying is what says the damage resolved at the
/// target that was named and not at the seat behind it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn reckless_assault_pays_a_mana_and_two_life_to_deal_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[reckless_assault(), forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let assault = on_battlefield(&engine, p0, reckless_assault()).expect("the Assault is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 for one damage to kill"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a Forest is tapped"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the untapped lands: with nothing floating the `{1}` is
    // unpayable and the line is not on the offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(assault, 0)),
        "`{{1}}` is not one, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Two Forests into the pool: the `{1}` the ability charges and one to
    // spare, so the mana the pool is missing afterwards is a payment rather
    // than a board that never held any.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and the Assault makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(assault, 0)),
        "with `{{1}}` floating the whole price is payable: {:?}",
        legal.abilities
    );
    assert!(
        !is_tapped(&engine, assault),
        "the price is a mana and two life, so the enchantment is never tapped"
    );

    activate(&mut engine, p0, reckless_assault(), 0);
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
        !options.contains(&assault),
        "the Assault is an enchantment and no creature, so it is no target for \
         its own ability: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards: both
    // halves of the price are still unpaid while this question stands.
    assert_eq!(
        engine.state().players[0].life,
        20,
        "Pay 2 life is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"Pay 2 life\" — two, and never a life per point of damage"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the damage is the resolution, not the cost: the target is still standing"
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
    assert!(
        on_battlefield(&engine, p0, reckless_assault()).is_some(),
        "the price was a mana and two life, so the enchantment stays to shoot again"
    );
}
