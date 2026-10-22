//! `cards/creatures/mv_3/seasoned_tactician.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn seasoned_tactician() -> CardIndex {
    card_index("d7c2f27c-8c82-4283-a17d-f979ee636542")
}

/// Seasoned Tactician ({2}{W} 1/3 Creature — Human Advisor):
/// - "{3}, Exile the top four cards of your library: The next time a source of your choice would deal damage to you this turn, prevent that damage."
#[test]
fn seasoned_tactician_prevents_next_damage_from_chosen_source_to_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[seasoned_tactician(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Attack P0 with Grizzly Bears; P0 activates Seasoned Tactician choosing Grizzly Bears.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, seasoned_tactician(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let source = *options.iter().find(|s| s.object == bears).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[0].life, 20);
}
