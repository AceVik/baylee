//! `cards/creatures/mv_5/anarchist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn anarchist() -> CardIndex {
    card_index("82b9ad93-df75-4417-9746-798db8fe6ba0")
}

/// Anarchist (`82b9ad93-df75-4417-9746-798db8fe6ba0`) — {4}{R} Creature — Human Wizard (Implemented):
/// "When this creature enters, you may return target sorcery card from your graveyard to your hand."
#[test]
fn anarchist_etb_returns_target_sorcery_from_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[anarchist(), index::FIREBALL])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let sorc = hand_to_graveyard(&mut engine, P0, index::FIREBALL);
    cast_from_hand(&mut engine, P0, anarchist());
    kit::settle(&mut engine, P0, |_, p| {
        if matches!(p, Pending::YesNo { .. }) {
            return Some(PlayerAction::YesNo(true));
        }
        kit::choose_targets(p, &[sorc], &[])
    });

    assert!(in_hand(&engine, P0, index::FIREBALL).is_some());
    assert!(in_graveyard(&engine, P0, index::FIREBALL).is_none());
}
