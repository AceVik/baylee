//! Oracle: “Whenever you cast an instant or sorcery spell, create a 1/1 red
//! Elemental creature token.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn young_pyromancer() -> CardIndex {
    card_index("5fac139a-07d3-4e6c-98e3-d98b199f7a6f")
}

/// Checklist: casting an instant and a sorcery each makes one token; a
/// creature spell does not. The token is a red 1/1 Elemental creature.
#[test]
fn young_pyromancer_triggers_for_instants_and_sorceries_only() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
            ],
        )
        .hand(
            0,
            &[
                young_pyromancer(),
                giant_growth(),
                counsel_of_the_soratami(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    cast_from_hand(&mut game, p0, young_pyromancer());
    pass_until(&mut game, stack_is_empty);
    let pyromancer = on_battlefield(&game, p0, young_pyromancer()).unwrap();
    assert!(
        tokens_of(&game, p0).is_empty(),
        "entering is not casting an instant or sorcery"
    );

    cast_from_hand(&mut game, p0, giant_growth());
    aim_at(&mut game, p0, pyromancer);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        tokens_of(&game, p0).len(),
        1,
        "one instant cast makes one token"
    );

    // The opponent casts a real spell after the active player passes; “you
    // cast” excludes a spell cast by another player.
    let p1 = PlayerId::new(1);
    game.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut game, p1, giant_growth());
    let opposing_elf = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    aim_at(&mut game, p1, opposing_elf);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        tokens_of(&game, p0).len(),
        1,
        "an opponent's instant is not one you cast"
    );

    // The next own main phase supplies fresh mana for the sorcery and a
    // non-triggering creature spell, each of which is cast for real.
    let first_turn = game.state().turn.number;
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::Priority { player, .. }
            if *player == p0
                && e.state().turn.active == p0
                && e.state().turn.number > first_turn
                && e.state().turn.phase == Phase::FirstMain)
    });
    cast_from_hand(&mut game, p0, counsel_of_the_soratami());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        tokens_of(&game, p0).len(),
        2,
        "one sorcery cast makes one more token"
    );

    cast_from_hand(&mut game, p0, llanowar_elves());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        tokens_of(&game, p0).len(),
        2,
        "a creature spell makes no token"
    );
    let token = tokens_of(&game, p0)[0];
    let definition = game.state().object(token).unwrap().token.unwrap();
    assert_eq!(definition.name, "Elemental");
    assert_eq!((definition.power, definition.toughness), (Some(1), Some(1)));
    assert!(definition.colors.contains(baylee_core::color::Color::Red));
    assert!(types(&game, token).contains(TypeSet::CREATURE));
}
