//! `cards/lands/fetch/sanctum_of_ugin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sanctum of Ugin is `Coverage::Implemented`.  It prints `{T}: Add {C}` as a
/// non-intrinsic mana ability (the land has no basic subtype), so the activation
/// sits in `legal.abilities` and not `legal.mana_abilities`.  Activating it puts
/// one colourless mana in the pool without using the stack.
///
/// This test strikes that single sentence: the offer lands in the right list,
/// pressing it taps the land and fills the pool with `{C}`, and the stack stays
/// empty.  The cast trigger is not exercised here because every colorless spell
/// with mana value 7 or greater available in the pool requires an X choice the
/// harness cannot answer through `pass_until`.
#[test]
fn sanctum_of_ugin_taps_for_one_colorless_via_printed_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[sanctum_of_ugin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctum = on_battlefield(&engine, p0, sanctum_of_ugin()).expect("the Sanctum is out");

    // The Sanctum prints no basic land type, so the CR 305.6 shortcut does not
    // apply: its mana ability is in `legal.abilities`, not `legal.mana_abilities`.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&sanctum),
        "no basic land type means no CR 305.6 shortcut: {:?}",
        legal.mana_abilities
    );
    assert!(
        legal.abilities.contains(&(sanctum, 0)),
        "the printed {{T}}: Add {{C}} is an ordinary activation in legal.abilities: {:?}",
        legal.abilities
    );

    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    let before_total = engine.state().players[0].mana_pool.total();
    activate(&mut engine, p0, sanctum_of_ugin(), 0);

    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack (CR 605.3b)"
    );
    assert!(
        is_tapped(&engine, sanctum),
        "the {{T}} in the cost tapped the land"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 1,
        "{{T}}: Add {{C}} added exactly one colourless"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        before_total + 1,
        "and nothing else came with it"
    );
}

/// Sanctum of Ugin: "Whenever you cast a colorless spell with mana value 7
/// or greater, you may sacrifice this land. If you do, search your library
/// for a colorless creature card, reveal it, put it into your hand, then
/// shuffle."
#[test]
fn sanctum_of_ugin_sacrifices_for_a_colorless_creature_when_a_big_colorless_spell_is_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4201, lair_ornithopter())
        .battlefield(
            0,
            &[
                sanctum_of_ugin(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[darksteel_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, darksteel_gargoyle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the may-sacrifice question");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the search asks for its card")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card came from the search");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, sanctum_of_ugin()).is_none(),
        "the Sanctum was sacrificed"
    );
    assert!(in_graveyard(&engine, p0, sanctum_of_ugin()).is_some());
    assert!(
        in_hand(&engine, p0, lair_ornithopter()).is_some(),
        "the colorless creature is in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 1,
        "the Gargoyle left the hand, the Thopter joined it"
    );
}

/// The same trigger answered "no" keeps the land and finds nothing.
#[test]
fn sanctum_of_ugin_declined_keeps_the_land_and_searches_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4202, lair_ornithopter())
        .battlefield(
            0,
            &[
                sanctum_of_ugin(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[darksteel_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, darksteel_gargoyle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declined");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sanctum_of_ugin()).is_some());
    assert!(in_hand(&engine, p0, lair_ornithopter()).is_none());
}
