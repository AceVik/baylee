//! `cards/sorceries/mv_1/psychic_purge.rs`, played: the sorcery half, "Psychic
//! Purge deals 1 damage to any target." Its discard trigger ("When a spell or
//! ability an opponent controls causes you to discard this card, that player
//! loses 5 life.") is played in `looting/discarded.rs`.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// p0 with an Island and the Purge in hand, p1 with a Grizzly Bears (2/2),
/// at p0's main phase with the Purge cast and its target question open.
fn purge_cast() -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[index::PSYCHIC_PURGE])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("p1's Bears are out");
    cast_from_hand(&mut engine, p0, index::PSYCHIC_PURGE);
    let Pending::ChooseTargets {
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
    assert_eq!((min, max), (1, 1), "one target, and the spell requires one");
    assert!(
        options.contains(&bears) && player_options.contains(&p0) && player_options.contains(&p1),
        "\"any target\": a creature or either player: {options:?} {player_options:?}"
    );
    (engine, bears)
}

/// "deals 1 damage to any target", aimed at the opponent: exactly one life,
/// and nothing for the caster.
#[test]
fn psychic_purge_deals_exactly_one_damage_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = purge_cast();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 was on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19, "1 damage, not 0 or 2");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the caster loses nothing"
    );
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert!(
        in_graveyard(&engine, p0, index::PSYCHIC_PURGE).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}

/// The same spell aimed at a creature: one damage marked on a 2/2, which
/// survives, and no life lost by either player.
#[test]
fn psychic_purge_deals_exactly_one_damage_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = purge_cast();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .expect("the Bears were on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        1,
        "1 damage marked, not 2: the 2/2 would have died"
    );
    assert_eq!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS),
        Some(bears),
        "a 2/2 survives 1 damage"
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
}
