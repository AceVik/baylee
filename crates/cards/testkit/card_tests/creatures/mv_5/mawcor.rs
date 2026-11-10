//! `cards/creatures/mv_5/mawcor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mawcor prints two lines — "Flying" and "{T}: This creature deals 1 damage to
/// any target" — and neither is visible in the card file, so both are played on
/// one board. The 3/3 flier is cast for `{3}{U}{U}` off five Islands and read for
/// its keyword and its printed body; the `{T}` then has to wait a full turn,
/// because a creature that just entered is summoning sick and its activated
/// ability is not offered at all (CR 302.6). "Any target" is one choice carrying
/// creatures *and* players (CR 115.4), so the Elf across the table is offered
/// beside both seats, and the single point is read on both life totals — it can
/// only be the target that paid, not the board and not the seat that aimed it.
#[test]
fn mawcor_flies_and_taps_to_deal_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[mawcor()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {3}{U}{U} out of five Islands, which leaves nothing behind: the cast is a
    // real payment and not a label on a free permanent.
    cast_from_hand(&mut engine, p0, mawcor());
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, mawcor()).expect("the Mawcor resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Islands paid {{3}}{{U}}{{U}} to the last mana"
    );
    assert_eq!(pt(&engine, beast), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, beast).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );

    // A creature that entered this turn has summoning sickness: its {{T}} is not
    // a price it may pay yet, which is why the damage line needs another turn.
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, beast),
        "the untap step stood the Mawcor back up"
    );

    activate(&mut engine, p0, mawcor(), 0);
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
    assert_eq!(player, p0, "the seat that activated aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&theirs),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays for it.
    assert!(
        !is_tapped(&engine, beast),
        "the {{T}} is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert!(is_tapped(&engine, beast), "{{T}} is the whole price");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has happened yet: the damage is the resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — one, to the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved, so the one point \
         went to the target and not to the board"
    );
}
