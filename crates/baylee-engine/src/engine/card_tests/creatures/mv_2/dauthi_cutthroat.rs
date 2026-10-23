//! `cards/creatures/mv_2/dauthi_cutthroat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dauthi_cutthroat() -> CardIndex {
    card_index("5fdde593-54cb-4186-974b-df4383f7efe2")
}

/// Dauthi Cutthroat (`5fdde593-54cb-4186-974b-df4383f7efe2`) — {1}{B} Creature — Dauthi Minion (Implemented):
/// "Shadow. {1}{B}, {T}: Destroy target creature with shadow."
#[test]
fn dauthi_cutthroat_has_shadow_and_destroys_target_creature_with_shadow() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[dauthi_cutthroat(), swamp(), swamp()])
        .battlefield(1, &[dauthi_cutthroat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let cutthroat = on_battlefield(&engine, P0, dauthi_cutthroat()).unwrap();
    let victim = on_battlefield(&engine, P1, dauthi_cutthroat()).unwrap();
    tap_all_mana(&mut engine, P0);
    kit::activate_object(&mut engine, P0, cutthroat, 0);
    aim_at(&mut engine, P0, victim);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, dauthi_cutthroat()).is_none());
    assert!(in_graveyard(&engine, P1, dauthi_cutthroat()).is_some());
}
