//! `cards/sorceries/mv_1/restless_dreams.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn restless_dreams() -> CardIndex {
    card_index("65f1dd09-0640-47f8-8bfd-7aceaa5646a6")
}

/// Restless Dreams (65f1dd09-0640-47f8-8bfd-7aceaa5646a6) — {B} Sorcery:
/// "As an additional cost to cast this spell, discard X cards. Return X target
/// creature cards from your graveyard to your hand."
#[test]
fn restless_dreams_discards_x_cards_and_returns_x_creatures_from_your_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[swamp()],
        &[],
        &[
            restless_dreams(),
            forest(),
            island(),
            llanowar_elves(),
            aurochs(),
            sol_ring(),
        ],
        &[fangren_hunter()],
    );
    let pitch1 = in_hand(&engine, P0, forest()).unwrap();
    let pitch2 = in_hand(&engine, P0, island()).unwrap();
    let gy_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let gy_beast = hand_to_graveyard(&mut engine, P0, aurochs());
    let gy_ring = hand_to_graveyard(&mut engine, P0, sol_ring());
    let their_gy = hand_to_graveyard(&mut engine, P1, fangren_hunter());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, restless_dreams());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert_eq!(options, vec![gy_elf, gy_beast]);
    assert!(!options.contains(&gy_ring) && !options.contains(&their_gy));
    lk::aim(&mut engine, &[gy_elf, gy_beast], &[]);

    for pitch in [pitch1, pitch2] {
        let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
            panic!("expected discard cost prompt, got {:?}", engine.pending());
        };
        assert_eq!(player, P0);
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![pitch],
                },
            )
            .unwrap();
    }
    assert_eq!(lk::zone_of(&engine, pitch1), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, pitch2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, gy_elf), Zone::Graveyard);

    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, gy_elf), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, gy_beast), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, gy_ring), Zone::Graveyard);
}
