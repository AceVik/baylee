//! `cards/lands/reveal/fortified_beachhead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fortified Beachhead prints `As this land enters, you may reveal a Soldier card
/// from your hand. This land enters tapped unless you revealed a Soldier card this
/// way or you control a Soldier.`, `{{T}}: Add {{W}} or {{U}}.`, and `{{5}}, {{T}}: Soldiers
/// you control get +1/+1 until end of turn.`
///
/// This test seats an `earth_king_s_lieutenant()`, plays Fortified Beachhead to
/// verify that it enters untapped, and activates ability 0 to choose white mana.
#[test]
fn fortified_beachhead_enters_untapped_with_soldier_and_adds_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[earth_king_s_lieutenant()])
        .hand(0, &[fortified_beachhead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fortified_beachhead());
    assert!(
        !entered_tapped(&engine, land),
        "enters untapped with a Soldier"
    );

    activate(&mut engine, p0, fortified_beachhead(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![ManaColor::White, ManaColor::Blue]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}

/// `{5}, {T}: Soldiers you control get +1/+1 until end of turn.` The pump
/// reaches your Soldiers and nothing else: not an opponent's Soldier, not your
/// own non-Soldier. It is a one-shot effect on the permanents there when it
/// resolves (CR 611.2c), so a Soldier that enters afterwards is not pumped,
/// and it is over once the turn is.
#[test]
fn fortified_beachhead_pump_reaches_only_your_soldiers_present_now_until_end_of_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fortified_beachhead(),
                index::ALABORN_GRENADIER,
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[index::ALABORN_GRENADIER])
        .hand(0, &[index::YOTIAN_SOLDIER])
        .start();
    keep_mulligans(&mut e);
    // The seeded land still asks its entry question; reveal nothing.
    if let Pending::ChooseCards {
        player,
        prompt: crate::choice::ChoicePrompt::RevealOrEnterTapped,
        ..
    } = e.pending().clone()
    {
        e.apply(player, PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
    }
    reach_main_phase(&mut e, p0);
    let beachhead = on_battlefield(&e, p0, fortified_beachhead()).unwrap();
    e.dev_state_mut(p0).unwrap().set_tapped(beachhead, false);
    let mine = on_battlefield(&e, p0, index::ALABORN_GRENADIER).unwrap();
    let theirs = on_battlefield(&e, p1, index::ALABORN_GRENADIER).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let (soldier, other_soldier, non_soldier) = (pt(&e, mine), pt(&e, theirs), pt(&e, elf));
    assert_eq!(soldier, other_soldier, "the two Soldiers start alike");

    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 5);
    e.refresh_offer();
    activate(&mut e, p0, fortified_beachhead(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, mine),
        (soldier.0 + 1, soldier.1 + 1),
        "a Soldier you control gets +1/+1"
    );
    assert_eq!(
        pt(&e, theirs),
        other_soldier,
        "\"Soldiers you control\": an opponent's Soldier is not pumped"
    );
    assert_eq!(
        pt(&e, elf),
        non_soldier,
        "\"Soldiers\": a creature of yours that is no Soldier is not pumped"
    );
    assert!(is_tapped(&e, beachhead), "{{T}} is part of the price");

    // CR 611.2c: the set of objects is fixed as the effect resolves.
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::YOTIAN_SOLDIER);
    pass_until(&mut e, stack_is_empty);
    let late = on_battlefield(&e, p0, index::YOTIAN_SOLDIER).expect("the Soldier resolved");
    assert_eq!(
        pt(&e, late),
        (1, 4),
        "a Soldier entering after the pump resolved is not pumped (CR 611.2c)"
    );
    assert_eq!(pt(&e, mine), (soldier.0 + 1, soldier.1 + 1));

    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        pt(&e, mine),
        soldier,
        "\"until end of turn\": gone when the turn is"
    );
    assert_eq!(pt(&e, late), (1, 4));
}
