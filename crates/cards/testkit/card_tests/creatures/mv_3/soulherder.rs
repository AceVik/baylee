//! `cards/creatures/mv_3/soulherder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soulherder prints two sentences that only mean anything together:
/// "Whenever a creature is exiled from the battlefield, put a +1/+1 counter on
/// this creature", and at the beginning of your end step, "you may exile
/// another target creature you control, then return that card to the
/// battlefield under its owner's control". One end step plays both, because the
/// blink *is* an exile from the battlefield — the counter is the proof the
/// creature really left, and the card standing on the battlefield afterwards is
/// the proof it really came back. The two bystanders make the filter readable
/// rather than assumed: the Soulherder itself is not "another", and the Elf
/// across the table is the same card and not "you control".
#[test]
fn soulherder_blinks_a_creature_at_end_of_turn_and_grows_for_the_exile() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), forest(), llanowar_elves()])
        .hand(0, &[soulherder()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W}{U} out of the Plains, the Island and the Forest — exactly three
    // mana — with the Elf named as the printing kept back, because it is the
    // creature the end step below is about.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three lands, three mana, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, soulherder());
    pass_until(&mut engine, stack_is_empty);

    let herder = on_battlefield(&engine, p0, soulherder()).expect("the Soulherder resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, herder),
        (1, 1),
        "a printed 1/1 before anything has been exiled"
    );

    // Through combat and the second main to the end step, where the second
    // printed sentence asks its question.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the seat whose end step it is names the target");
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you may … another target creature\": none or one, and no more"
    );
    assert!(
        options.contains(&elf),
        "another creature you control is exactly the menu: {options:?}"
    );
    assert!(
        !options.contains(&herder),
        "\"another\": the Soulherder cannot blink itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\": the Elf across the table is the same card and not \
         yours to blink: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"then return that card to the battlefield under its owner's control\" — \
         the exiled creature is back"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        0,
        "and it is not still in exile: a blink that never came back would leave \
         a card sitting there"
    );
    assert_eq!(
        pt(&engine, herder),
        (2, 2),
        "\"whenever a creature is exiled from the battlefield, put a +1/+1 \
         counter on this creature\" — the blink's own exile is the only exile in \
         this game"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability never named did not move"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it carries no counter for somebody else's blink"
    );
}
