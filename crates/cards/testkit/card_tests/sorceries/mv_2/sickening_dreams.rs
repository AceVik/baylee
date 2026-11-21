//! `cards/sorceries/mv_2/sickening_dreams.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sickening_dreams() -> CardIndex {
    card_index("728d2e30-2f36-4054-81db-29b01e376a82")
}

/// Sickening Dreams (728d2e30-2f36-4054-81db-29b01e376a82) — {1}{B} Sorcery:
/// "As an additional cost to cast this spell, discard X cards. Sickening Dreams
/// deals X damage to each creature and each player."
#[test]
fn sickening_dreams_discards_x_cards_and_deals_x_damage_to_creatures_and_players() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), aurochs()],
        &[fangren_hunter()],
        &[sickening_dreams(), forest(), island(), plains()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_hunter = lk::obj(&engine, P1, fangren_hunter());
    let c1 = in_hand(&engine, P0, forest()).unwrap();
    let c2 = in_hand(&engine, P0, island()).unwrap();
    let c3 = in_hand(&engine, P0, plains()).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, sickening_dreams());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    for card in [c1, c2, c3] {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![card],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(
        lk::zone_of(&engine, my_beast),
        Zone::Graveyard,
        "2/3 dies to 3"
    );
    assert_eq!(lk::zone_of(&engine, their_hunter), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, their_hunter), 3);
    assert_eq!((lk::life(&engine, P0), lk::life(&engine, P1)), (17, 17));
}
