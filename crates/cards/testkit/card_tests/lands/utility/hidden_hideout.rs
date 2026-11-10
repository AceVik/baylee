//! `cards/lands/utility/hidden_hideout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hidden Hideout prints `This land enters tapped.`, `{{T}}: Add one mana of any color in your commander's color identity.`, and `{{2}}, {{T}}: Target creature you control with a counter on it gains lifelink until end of turn.`
///
/// Under `Coverage::Partial`, the land enters tapped and taps for mana in the commander's color identity, while the lifelink ability is omitted because filters cannot check for counters.
/// With a mono-black commander (`sheoldred_the_apocalypse()`), the land enters tapped, untaps on the following turn, and with `{{2}}` floating from two `forest()` lands and a creature present (`quiet_creature()`), ability 1 is not offered in `legal.abilities`.
/// Activating ability 0 produces one black mana matching the commander's color identity.
#[test]
fn hidden_hideout_enters_tapped_and_produces_commander_color_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[forest(), forest(), quiet_creature()])
        .hand(0, &[hidden_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_hideout());
    assert!(
        entered_tapped(&engine, land),
        "hidden hideout enters tapped"
    );

    // Advance to next turn so the land untaps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    // Float the two Forests and the Elves' own {{G}} while keeping Hidden
    // Hideout untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "ability 0 (commander identity mana) is offered"
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "with {{2}} floating, a creature present, and land untapped, ability 1 is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, hidden_hideout(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, land));
}

#[test]
fn hideout_accepts_any_counter_kind_but_only_on_your_creature() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::HIDDEN_HIDEOUT,
                index::GRIZZLY_BEARS,
                index::MIND_STONE,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::HIDDEN_HIDEOUT).unwrap();
    let own = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let stone = on_battlefield(&e, p0, index::MIND_STONE).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(land, false);
    for id in [own, other, stone] {
        state
            .object_mut(id)
            .unwrap()
            .counters
            .add(CounterKind::Charge, 1);
    }
    state.invalidate_projections();
    state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::HIDDEN_HIDEOUT, 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("targets")
    };
    assert_eq!(options, &vec![own]);
    aim_at(&mut e, p0, own);
    pass_until(&mut e, stack_is_empty);
    assert!(keywords(&e, own).contains(KeywordSet::LIFELINK));
    reach_their_main_phase(&mut e, p1);
    assert!(!keywords(&e, own).contains(KeywordSet::LIFELINK));
}
