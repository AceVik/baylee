//! `cards/sorceries/mv_4/persecute.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn persecute() -> CardIndex {
    card_index("676c4cfa-f929-43aa-95cc-df81a596c2a2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Persecute (676c4cfa-f929-43aa-95cc-df81a596c2a2) — {2}{B}{B} Sorcery:
/// "Choose a color. Target player reveals their hand and discards all cards of
/// that color."
#[test]
fn persecute_reveals_hand_and_discards_all_cards_including_multicolor_of_chosen_color() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), swamp()],
        &[],
        &[persecute()],
        &[
            festering_goblin(),
            lotleth_troll(),
            grizzly_bears(),
            sol_ring(),
            swamp(),
        ],
    );
    let mono_black = in_hand(&engine, P1, festering_goblin()).unwrap();
    let black_green = in_hand(&engine, P1, lotleth_troll()).unwrap();
    let mono_green = in_hand(&engine, P1, grizzly_bears()).unwrap();
    let colorless = in_hand(&engine, P1, sol_ring()).unwrap();
    let land = in_hand(&engine, P1, swamp()).unwrap();

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, persecute(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, P0);
    assert_eq!(options.len(), 5, "all five colors offered");

    let before_entries = engine.journal().entries().len();
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    lk::settle(&mut engine);

    let entries = &engine.journal().entries()[before_entries..];
    for card in [mono_black, black_green, mono_green, colorless, land] {
        assert!(
            entries.iter().any(
                |e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&card))
            ),
            "entire hand must be revealed, missing {card:?}"
        );
    }
    assert_eq!(lk::zone_of(&engine, mono_black), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, black_green), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, mono_green), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, colorless), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, land), Zone::Hand);
}
