//! `cards/instants/mv_5/omen_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SAVANNAH_LIONS;

fn omen_of_fire() -> CardIndex {
    card_index("5935e9b3-b9b9-413c-ba17-103f5792131a")
}

/// Omen of Fire ({3}{R}{R} Instant, Partial):
/// "Return all Islands to their owners' hands." The second sentence, "Each
/// player sacrifices a Plains or a white permanent of their choice for each
/// white permanent they control", is not played: its count is per player.
/// P0 here controls a Plains and no white permanent, so the real card has
/// them sacrifice nothing, and so does this one.
#[test]
fn omen_of_fire_returns_every_island_to_its_owner_and_sacrifices_nothing_without_white() {
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
        .battlefield(1, &[island(), plains(), mountain(), SAVANNAH_LIONS])
        .hand(0, &[omen_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, island()).is_some());
    assert!(on_battlefield(&engine, p1, island()).is_some());

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, omen_of_fire());
    pass_until(&mut engine, stack_is_empty);

    // All Islands, whoever controls them, go to their owners' hands.
    assert!(on_battlefield(&engine, p0, island()).is_none());
    assert!(on_battlefield(&engine, p1, island()).is_none());
    assert!(in_hand(&engine, p0, island()).is_some());
    assert!(in_hand(&engine, p1, island()).is_some());
    // Nothing else moves: Mountains are no Islands, and a Plains is no white
    // permanent, so P0 (no white permanent) sacrifices nothing.
    assert!(on_battlefield(&engine, p0, mountain()).is_some());
    assert!(on_battlefield(&engine, p1, mountain()).is_some());
    assert!(
        on_battlefield(&engine, p0, plains()).is_some(),
        "a Plains without a white permanent beside it is not sacrificed"
    );
    assert!(in_graveyard(&engine, p0, plains()).is_none());
}
