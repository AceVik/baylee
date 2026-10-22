//! `cards/artifacts/mv_1/cursed_scroll.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cursed_scroll() -> CardIndex {
    card_index("514752ea-d453-41d4-97cf-b1dcdb4b4a08")
}

/// Cursed Scroll (514752ea-d453-41d4-97cf-b1dcdb4b4a08) — {1} Artifact (Partial):
/// "{3}, {T}: Choose a card name, then reveal a card at random from your hand. If that card has the chosen name, this artifact deals 2 damage to any target."
#[test]
fn cursed_scroll_chooses_card_name_reveals_random_card_and_deals_two_damage() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[cursed_scroll(), mountain(), mountain(), mountain()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, cursed_scroll(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: index::GRIZZLY_BEARS,
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}
