//! `cards/sorceries/mv_2/nostalgic_dreams.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nostalgic_dreams() -> CardIndex {
    card_index("fabbc474-682c-4565-861c-d41de05abb8f")
}

/// Nostalgic Dreams (fabbc474-682c-4565-861c-d41de05abb8f) — {G}{G} Sorcery:
/// "As an additional cost to cast this spell, discard X cards. Return X target
/// cards from your graveyard to your hand. Exile Nostalgic Dreams."
#[test]
fn nostalgic_dreams_discards_x_returns_x_cards_to_hand_and_exiles_itself() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest()],
        &[],
        &[
            nostalgic_dreams(),
            plains(),
            island(),
            sol_ring(),
            counterspell(),
        ],
        &[aurochs()],
    );
    let dreams = in_hand(&engine, P0, nostalgic_dreams()).unwrap();
    let pitch1 = in_hand(&engine, P0, plains()).unwrap();
    let pitch2 = in_hand(&engine, P0, island()).unwrap();
    let gy_ring = hand_to_graveyard(&mut engine, P0, sol_ring());
    let gy_spell = hand_to_graveyard(&mut engine, P0, counterspell());
    let their_gy = hand_to_graveyard(&mut engine, P1, aurochs());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, nostalgic_dreams());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert_eq!(options, vec![gy_ring, gy_spell]);
    assert!(!options.contains(&their_gy));
    lk::aim(&mut engine, &[gy_ring, gy_spell], &[]);

    for pitch in [pitch1, pitch2] {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![pitch],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, gy_ring), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, gy_spell), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, pitch1), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, pitch2), Zone::Graveyard);
    assert_eq!(
        lk::zone_of(&engine, dreams),
        Zone::Exile,
        "Exile Nostalgic Dreams"
    );
}
