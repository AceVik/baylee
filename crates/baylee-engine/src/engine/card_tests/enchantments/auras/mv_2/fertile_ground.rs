#[allow(clippy::wildcard_imports)]
use super::*;

fn fertile_ground() -> CardIndex {
    card_index("cf14d4e5-5965-45ad-97f7-26facf2884b5")
}

#[test]
fn fertile_ground_adds_bonus_mana_when_enchanted_land_is_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[fertile_ground()])
        .battlefield(0, &[forest(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, plains()).expect("seated");
    for f in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, fertile_ground());
    aim_at(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}
