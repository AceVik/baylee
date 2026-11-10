//! `cards/creatures/mv_4/witch_hunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn witch_hunter() -> CardIndex {
    card_index("5fc1f04a-2c39-4b3a-b62e-6e30c3d8a583")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
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

fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[witch_hunter(), savannah_lions()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Witch Hunter (5fc1f04a-2c39-4b3a-b62e-6e30c3d8a583) — "{T}: This creature
/// deals 1 damage to target player or planeswalker."
#[test]
fn witch_hunter_pings_a_player_but_not_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let hunter = on_battlefield(&engine, p0, witch_hunter()).expect("the Hunter is out");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");
    float(&mut engine, 0, &[(ManaColor::White, 3)]);

    activate(&mut engine, p0, witch_hunter(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&bears),
        "a creature is neither a player nor a planeswalker: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19, "1 damage");
    assert_eq!(
        engine.state().object(bears).expect("the Bears live").damage,
        0
    );
    assert!(is_tapped(&engine, hunter), "{{T}} is part of the cost");
    assert!(
        !offered(&engine, witch_hunter(), 0) && !offered(&engine, witch_hunter(), 1),
        "tapped: neither ability can be used again this turn"
    );
}

/// "{1}{W}{W}, {T}: Return target creature an opponent controls to its
/// owner's hand."
#[test]
fn witch_hunter_returns_an_opposing_creature_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let hunter = on_battlefield(&engine, p0, witch_hunter()).expect("the Hunter is out");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("my Lions are out");
    float(
        &mut engine,
        0,
        &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
    );

    activate(&mut engine, p0, witch_hunter(), 1);
    let options = aim_at(&mut engine, p0, bears);
    assert!(
        options.contains(&bears),
        "their creature is a target: {options:?}"
    );
    assert!(
        !options.contains(&lions) && !options.contains(&hunter),
        "only creatures an opponent controls: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "the Bears left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, grizzly_bears()).is_some(),
        "and are in their owner's hand"
    );
    assert!(is_tapped(&engine, hunter), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{W}}{{W}} paid"
    );
}
