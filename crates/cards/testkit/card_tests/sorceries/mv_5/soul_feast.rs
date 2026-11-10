//! `cards/sorceries/mv_5/soul_feast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soul Feast — {3}{B}{B} sorcery: "Target player loses 4 life and you gain 4
/// life."
///
/// The two clauses name two *different* players, and that is the whole card:
/// the spell is aimed across the table, so the opponent's life falls by four
/// while the caster's rises by four. A reading that had lost "you" would leave
/// the caster at twenty, and one that had let the loss land on the caster
/// would net nothing at all. The menu is read while it stands, because "target
/// player" is a target (CR 115.1) that no permanent can answer, and the five
/// Swamps are tapped first: `can_afford` reads the pool and not the untapped
/// lands.
#[test]
fn soul_feast_drains_the_player_it_names_and_heals_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 5])
        .hand(0, &[soul_feast()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "{{3}}{{B}}{{B}} is five mana and the five Swamps are the whole board"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5,
        "a Swamp makes black, which is the colour the two black symbols ask for"
    );

    cast_with_floating(&mut engine, p0, soul_feast());
    // A target that can only ever be a player asks `ChoosePlayer` rather
    // than a `ChooseTargets` with an empty object list.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is a player choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "\"target player\" is any player, its own controller included: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent was one of the players the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "\"target player loses 4 life\" — the seat that was named, and not the caster"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"and you gain 4 life\" — the caster, and not the seat that lost the four"
    );
    assert!(
        in_graveyard(&engine, p0, soul_feast()).is_some(),
        "a sorcery goes to its owner's graveyard as it resolves"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the five Swamps paid the {{3}}{{B}}{{B}} out of the pool"
    );
}
