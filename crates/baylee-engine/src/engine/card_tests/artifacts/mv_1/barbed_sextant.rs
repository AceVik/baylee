//! `cards/artifacts/mv_1/barbed_sextant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barbed_sextant() -> CardIndex {
    card_index("97ba11a1-16cd-47ee-a187-1e919c468e4e")
}

/// Barbed Sextant (97ba11a1-16cd-47ee-a187-1e919c468e4e) — {1} Artifact:
/// - "{1}, {T}, Sacrifice Barbed Sextant: Add one mana of any color. Draw a
///   card at the beginning of the next turn's upkeep."
#[test]
fn barbed_sextant_sacrifices_for_one_mana_of_any_color_and_draws_on_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[barbed_sextant(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, barbed_sextant(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor for Barbed Sextant, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choose Blue");

    // Mana ability resolves immediately without using the stack.
    assert!(on_battlefield(&engine, P0, barbed_sextant()).is_none());
    assert!(in_graveyard(&engine, P0, barbed_sextant()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "does not draw immediately"
    );

    // Advance to the next turn's upkeep (P1's upkeep) and let the delayed trigger resolve.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1,
        "draws a card at the beginning of the next turn's upkeep"
    );
}
