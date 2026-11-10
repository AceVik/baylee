//! `cards/sorceries/mv_5/natural_spring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Natural Spring is a {3}{G}{G} sorcery under `Coverage::Implemented` that causes target player to gain 8 life.
/// When cast, players in the game are presented as legal targets through `player_options`.
/// Targeting the casting player and resolving the spell increases their life total by exactly 8.
/// The other player's life total remains unaffected.
#[test]
fn natural_spring_gives_target_player_eight_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[natural_spring()])
        .life(0, 12)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, natural_spring()).expect("Natural Spring is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{3}}{{G}}{{G}}"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests produce five green mana"
    );
    cast_with_floating(&mut engine, p0, natural_spring());

    // A target that can only ever be a player asks `ChoosePlayer` rather
    // than a `ChooseTargets` with an empty object list.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("expected a player choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "caster chooses target");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "both players are offered as targets: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("targeting p0 is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "controller gained 8 life, increasing from 12 to 20"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent life remained 20"
    );
    assert!(
        in_graveyard(&engine, p0, natural_spring()).is_some(),
        "Natural Spring went to graveyard after resolution"
    );
}
