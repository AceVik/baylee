//! `cards/sorceries/mv_2/scent_of_cinder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn scent_of_cinder() -> CardIndex {
    card_index("ff50d002-834e-4b5e-8db0-b0c8b4efb21f")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Scent of Cinder (ff50d002-834e-4b5e-8db0-b0c8b4efb21f) — {1}{R} Sorcery:
/// "Reveal any number of red cards in your hand. Scent of Cinder deals X damage
/// to any target, where X is the number of cards revealed this way."
#[test]
fn scent_of_cinder_reveals_chosen_red_cards_and_deals_that_much_damage() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[],
        &[
            scent_of_cinder(),
            hill_giant(),
            wild_colos(),
            spikefield_hazard(),
            counterspell(),
            mountain(),
        ],
        &[],
    );
    let red1 = in_hand(&engine, P0, hill_giant()).unwrap();
    let red2 = in_hand(&engine, P0, wild_colos()).unwrap();
    let red3 = in_hand(&engine, P0, spikefield_hazard()).unwrap();
    let blue = in_hand(&engine, P0, counterspell()).unwrap();
    let land = in_hand(&engine, P0, mountain()).unwrap();

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, scent_of_cinder(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P0);
    assert_eq!(min, 0, "any number may be zero");
    assert_eq!(options.len(), 3, "only red cards in hand are offered");
    assert!(options.contains(&red1) && options.contains(&red2) && options.contains(&red3));
    assert!(!options.contains(&blue) && !options.contains(&land));

    let before_entries = engine.journal().entries().len();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![red1, red2],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    let entries = &engine.journal().entries()[before_entries..];
    for r in [red1, red2] {
        assert!(
            entries.iter().any(
                |e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&r))
            )
        );
        assert_eq!(
            lk::zone_of(&engine, r),
            Zone::Hand,
            "revealed stays in hand"
        );
    }
    assert_eq!(lk::life(&engine, P1), 18, "2 cards revealed -> 2 damage");
}
