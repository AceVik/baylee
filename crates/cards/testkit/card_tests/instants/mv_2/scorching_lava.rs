//! `cards/instants/mv_2/scorching_lava.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|p| p.index == card))
        })
}

fn scorching_lava() -> CardIndex {
    card_index("e42fb51e-254a-43ac-ad02-9f0fad0f4c8a")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Scorching Lava ({1}{R} Instant, Kicker {R}):
/// "Scorching Lava deals 2 damage to any target. If this spell was kicked, that
/// creature can't be regenerated this turn and if it would die this turn, exile it instead."
#[test]
fn scorching_lava_deals_two_and_kicked_prevents_regeneration_and_exiles_on_death() {
    // Unkicked kills Lotleth Troll? No, Lotleth Troll can regenerate and goes to GY if destroyed.
    // Kicked blocks Lotleth Troll's regeneration and exiles it instead of going to graveyard!
    let mut kicked = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[swamp(), swamp(), lotleth_troll()],
        &[scorching_lava()],
        &[],
    );
    let troll = lk::obj(&kicked, P1, lotleth_troll());

    // Opponent puts regeneration shield on Lotleth Troll first!
    reach_their_main_phase(&mut kicked, P1);
    tap_all_mana(&mut kicked, P1);
    activate(&mut kicked, P1, lotleth_troll(), 1);
    pass_until(&mut kicked, stack_is_empty);

    reach_their_main_phase(&mut kicked, P0);
    tap_all_mana(&mut kicked, P0);
    cast_with_floating(&mut kicked, P0, scorching_lava());
    kicked.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut kicked, &[troll], &[]);
    pass_until(&mut kicked, stack_is_empty);

    assert!(
        on_battlefield(&kicked, P1, lotleth_troll()).is_none(),
        "regeneration failed"
    );
    assert!(in_graveyard(&kicked, P1, lotleth_troll()).is_none());
    assert!(
        in_exile(&kicked, P1, lotleth_troll()),
        "exiled instead of dying"
    );

    // Unkicked can also hit a player for 2.
    let mut unkicked = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[],
        &[scorching_lava()],
        &[],
    );
    tap_all_mana(&mut unkicked, P0);
    cast_with_floating(&mut unkicked, P0, scorching_lava());
    unkicked.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut unkicked, &[], &[P1]);
    pass_until(&mut unkicked, stack_is_empty);
    assert_eq!(lk::life(&unkicked, P1), 18);
}
