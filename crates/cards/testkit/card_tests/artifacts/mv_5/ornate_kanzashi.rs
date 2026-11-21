//! `cards/artifacts/mv_5/ornate_kanzashi.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ornate_kanzashi() -> CardIndex {
    card_index("3c41c666-5cf4-4f35-9cb8-c41f1f78f31e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Ornate Kanzashi (3c41c666-5cf4-4f35-9cb8-c41f1f78f31e) — {5} Artifact:
/// "{2}, {T}: Target opponent exiles the top card of their library. You may
///  play that card this turn."
///
/// Targets only opponents, exiles the top card of their library, lets YOU play
/// it this turn (tested for both a spell and a land), and expires at end of
/// turn.
#[test]
fn ornate_kanzashi_exiles_opponents_top_card_for_you_to_play_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, grizzly_bears())
        .battlefield(
            0,
            &[ornate_kanzashi(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Activate targeting opponent p1: exiles top Grizzly Bears from p1's library.
    let their_top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    let my_forests = forests(&engine, p0, 4);
    tap_mana_where(&mut engine, p0, |id| my_forests[..2].contains(&id));
    activate(&mut engine, p0, ornate_kanzashi(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1) && !player_options.contains(&p0),
        "target opponent only: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(their_top).unwrap().zone, Zone::Exile);

    // p0 casts the opponent's exiled Grizzly Bears this turn using p0's remaining 2 Forests.
    tap_mana_where(&mut engine, p0, |id| my_forests[2..4].contains(&id));
    engine
        .apply(p0, PlayerAction::CastSpell { card: their_top })
        .expect("p0 may play the opponent's exiled card this turn");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(their_top).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(their_top).unwrap().controller, p0);
}

/// The play permission includes lands and expires at end of turn.
#[test]
fn ornate_kanzashi_allows_playing_exiled_land_and_expires_after_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ornate_kanzashi(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. Exile top Forest of p1's library and play it as a land this turn.
    let exiled_land = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ornate_kanzashi(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
        .apply(p0, PlayerAction::PlayLand { card: exiled_land })
        .expect("'play' permission includes playing an exiled land");
    assert_eq!(
        engine.state().object(exiled_land).unwrap().zone,
        Zone::Battlefield
    );

    // 2. Next turn: exile another Forest, do NOT play it; on the turn after, permission expired.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let missed = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| my_forests.contains(&id));
    activate(&mut engine, p0, ornate_kanzashi(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: missed })
            .is_err(),
        "play permission expired at the end of the turn it was exiled"
    );
}
