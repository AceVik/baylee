//! `cards/enchantments/mv_4/emrakul_s_influence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn emrakul_s_influence() -> CardIndex {
    card_index("92eb07ca-ec57-48c3-98ce-9e878bd968c1")
}

fn eldrazi_devastator() -> CardIndex {
    card_index("bc08ac66-969c-4905-a2d6-16ff565be475")
}

fn drowner_of_hope() -> CardIndex {
    card_index("2b3f2e85-5ea4-44d6-86e8-e8ce6d4f98c0")
}

fn hand_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// "Whenever you cast an Eldrazi creature spell with mana value 7 or
/// greater, draw two cards": an Eldrazi of mana value 8 draws two, one of
/// mana value 6 draws nothing.
#[test]
fn emrakul_s_influence_draws_two_for_a_big_eldrazi_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[emrakul_s_influence()])
        .hand(0, &[eldrazi_devastator(), drowner_of_hope()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Blue, 6);
    cast_with_floating(&mut engine, p0, drowner_of_hope());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_len(&engine, p0),
        1,
        "mana value 6: only the spell left the hand"
    );

    float(&mut engine, p0, ManaColor::Green, 8);
    cast_with_floating(&mut engine, p0, eldrazi_devastator());
    assert!(!stack_is_empty(&engine));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p0), 2, "mana value 8: two cards drawn");
    assert!(on_battlefield(&engine, p0, eldrazi_devastator()).is_some());
}
