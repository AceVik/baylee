//! `cards/enchantments/mv_4/vigilante_justice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vigilante_justice() -> CardIndex {
    card_index("9610176a-dcd6-4117-89db-45f56b16cdd6")
}

fn thunderscape_apprentice() -> CardIndex {
    card_index("34e325f0-b66f-44b9-a25c-da88357dbac4")
}

/// "Whenever a Human you control enters, this enchantment deals 1 damage to
/// any target.": a Human of mine entering asks for a target and the named
/// player takes 1; a Bears entering asks nothing.
#[test]
fn vigilante_justice_pings_any_target_when_my_human_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vigilante_justice()])
        .hand(0, &[grizzly_bears(), thunderscape_apprentice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 20, "a Bears is no Human");

    float(&mut engine, p0, ManaColor::Red, 1);
    cast_with_floating(&mut engine, p0, thunderscape_apprentice());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "1 damage to the target");
    assert_eq!(life_of(&engine, p0), 20);
}
