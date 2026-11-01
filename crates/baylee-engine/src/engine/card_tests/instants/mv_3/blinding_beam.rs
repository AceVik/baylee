//! `cards/instants/mv_3/blinding_beam.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn blinding_beam() -> CardIndex {
    card_index("0766cf02-ea55-4271-8c73-213c93b0baac")
}

/// Blinding Beam — {2}{W} Instant
/// "Choose one —
/// • Tap two target creatures.
/// • Creatures don't untap during target player's next untap step.
/// Entwine {1}"
#[test]
fn blinding_beam_entwined_taps_two_creatures_and_prevents_next_untap() {
    use crate::engine::card_tests::partials_w1b_kit as kit;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[blinding_beam()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears(), HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, blinding_beam());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let entwine_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Modes(3))
        .expect("entwine mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(entwine_idx))
        .unwrap();

    // Mode 1: tap two target creatures.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear, giant],
                players: vec![],
            },
        )
        .unwrap();
    // Mode 2: target player (P1).
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

    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, giant));

    // Walk to P1's main phase: P1's creatures do not untap during P1's next untap step.
    kit::walk_to_next_own_main(&mut engine, p1);
    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, giant));
}
