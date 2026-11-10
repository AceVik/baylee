//! `cards/creatures/mv_4/derelor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn derelor() -> CardIndex {
    card_index("43f2969e-46eb-4fd4-8b2d-ff81aafcc24a")
}

fn mindstab_thrull() -> CardIndex {
    card_index("43abe1f3-7b92-411a-bf93-e158f949685c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Derelor (43f2969e-46eb-4fd4-8b2d-ff81aafcc24a) — "Black spells you cast
/// cost {B} more to cast."
///
/// Mindstab Thrull is {1}{B}{B}: with Derelor out, {1}{B}{B} is not enough
/// and {1}{B}{B}{B} is, and pays exactly that.
#[test]
fn derelor_makes_your_black_spells_cost_one_black_more() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[mindstab_thrull()])
        .battlefield(0, &[derelor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    float(
        &mut engine,
        0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    let thrull = in_hand(&engine, p0, mindstab_thrull()).expect("the Thrull is in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: thrull })
            .is_err(),
        "{{1}}{{B}}{{B}} is its printed cost and no longer enough"
    );

    float(&mut engine, 0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, p0, mindstab_thrull());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}}{{B}}{{B}} was the whole price"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, mindstab_thrull()).is_some(),
        "the taxed spell resolved"
    );
}

/// The tax is {B}: black mana, not generic. {1}{B}{B} plus a fourth generic
/// mana does not pay it.
#[test]
fn derelor_asks_for_black_mana_not_generic() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[mindstab_thrull()])
        .battlefield(0, &[derelor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    float(
        &mut engine,
        0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
    );
    let thrull = in_hand(&engine, p0, mindstab_thrull()).expect("the Thrull is in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: thrull })
            .is_err(),
        "a fourth mana that is not black does not pay for {{B}} more"
    );
}

/// A spell that is not black costs what it printed.
#[test]
fn derelor_leaves_a_non_black_spell_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[grizzly_bears()])
        .battlefield(0, &[derelor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    float(
        &mut engine,
        0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut engine, p0, grizzly_bears());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}

/// "You cast": the opponent's black spells are not taxed.
#[test]
fn derelor_does_not_tax_the_opponents_black_spells() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[derelor()])
        .hand(1, &[mindstab_thrull()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    float(
        &mut engine,
        1,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, p1, mindstab_thrull());
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, mindstab_thrull()).is_some(),
        "cast for its printed cost"
    );
}
