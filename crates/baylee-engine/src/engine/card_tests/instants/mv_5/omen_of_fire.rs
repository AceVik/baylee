//! `cards/instants/mv_5/omen_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SAVANNAH_LIONS;

fn omen_of_fire() -> CardIndex {
    card_index("5935e9b3-b9b9-413c-ba17-103f5792131a")
}

/// Omen of Fire ({3}{R}{R} Instant, Partial):
/// "Return all Islands to their owners' hands. Each player sacrifices a Plains or a white permanent of their choice for each white permanent they control."
#[test]
fn omen_of_fire_bounces_all_islands_and_each_player_sacrifices_plains_or_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                plains(),
            ],
        )
        .battlefield(1, &[island(), plains(), SAVANNAH_LIONS])
        .hand(0, &[omen_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_plains = on_battlefield(&engine, p1, plains()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, omen_of_fire());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    while let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        let chosen = if player == p1 { p1_plains } else { options[0] };
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
        });
    }

    assert!(on_battlefield(&engine, p0, island()).is_none());
    assert!(on_battlefield(&engine, p1, island()).is_none());
    assert!(in_graveyard(&engine, p0, plains()).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
}
