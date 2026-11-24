//! `cards/sorceries/mv_2/addle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn addle() -> CardIndex {
    card_index("3745aea4-4455-4859-9772-bcfd14e4067b")
}

/// Addle reveals the whole hand, but its caster chooses exactly one card of
/// the color chosen on resolution; every other card remains in hand.
#[test]
fn addle_reveals_the_hand_and_discards_only_the_chosen_color_card() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp()],
        &[],
        &[addle()],
        &[lightning_bolt(), llanowar_elves(), sol_ring(), swamp()],
    );
    let bolt = in_hand(&engine, P1, lightning_bolt()).unwrap();
    let elf = in_hand(&engine, P1, llanowar_elves()).unwrap();
    let ring = in_hand(&engine, P1, sol_ring()).unwrap();
    let land = in_hand(&engine, P1, swamp()).unwrap();

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, addle(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, P0);
    assert_eq!(options.len(), 5, "all five colors are offered");

    let before_entries = engine.journal().entries().len();
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((player, min, max), (P0, 1, 1));
    assert_eq!(options, vec![bolt], "only the red card can be chosen");
    let revealed = &engine.journal().entries()[before_entries..];
    for card in [bolt, elf, ring, land] {
        assert!(
            revealed.iter().any(
                |entry| matches!(&entry.event, GameEvent::Revealed { player, cards }
                    if *player == P1 && cards.contains(&card))
            ),
            "the whole hand is revealed, missing {card:?}"
        );
    }
    for objects in [vec![elf], vec![ring], vec![land], vec![bolt, elf]] {
        assert!(
            engine
                .apply(P0, PlayerAction::ChooseObjects { objects })
                .is_err(),
            "a card of another color or more than one card is not a legal answer"
        );
    }

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());
    for card in [llanowar_elves(), sol_ring(), swamp()] {
        assert!(in_hand(&engine, P1, card).is_some());
        assert!(in_graveyard(&engine, P1, card).is_none());
    }
}
