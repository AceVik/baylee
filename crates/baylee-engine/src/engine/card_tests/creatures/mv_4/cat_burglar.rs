//! `cards/creatures/mv_4/cat_burglar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cat_burglar() -> CardIndex {
    card_index("16aea5b3-9f43-4852-b8b8-80e87909b794")
}

/// Cat Burglar (`16aea5b3-9f43-4852-b8b8-80e87909b794`) — {3}{B} Creature — Kor Rogue Minion (Implemented):
/// "{2}{B}, {T}: Target player discards a card. Activate only as a sorcery."
#[test]
fn cat_burglar_causes_target_player_to_discard_at_sorcery_speed() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cat_burglar(), swamp(), swamp(), swamp()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let burglar = on_battlefield(&engine, P0, cat_burglar()).unwrap();
    let card_in_p1_hand = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    kit::activate_object(&mut engine, P0, burglar, 0);
    kit::settle(&mut engine, P0, |_, p| {
        kit::choose_targets(p, &[], &[P1]).or_else(|| kit::choose_cards(p, &[card_in_p1_hand]))
    });

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
