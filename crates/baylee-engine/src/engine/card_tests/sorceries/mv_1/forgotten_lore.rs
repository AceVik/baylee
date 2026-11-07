//! `cards/sorceries/mv_1/forgotten_lore.rs`, played.
//!
//! Oracle:
//! - "Target opponent chooses a card in your graveyard. You may pay {G}. If you do, repeat this process except that opponent can't choose a card already chosen for Forgotten Lore. Then put the last chosen card into your hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn forgotten_lore() -> CardIndex {
    card_index("ca9f538d-4191-448a-870d-f645b7c37c1b")
}

/// Casts at the opponent with `extra_green` left after the spell's own cost.
/// The graveyard cards and their order are controlled by the test, not drawn.
fn cast(cards: &[CardIndex], extra_green: u32) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let mut hand = vec![forgotten_lore()];
    hand.extend_from_slice(cards);
    let mut engine = lk::game(SEED, &[], &[], &hand, &[]);
    let buried = cards
        .iter()
        .map(|card| hand_to_graveyard(&mut engine, P0, *card))
        .collect();
    lk::float(&mut engine, P0, &[(ManaColor::Green, extra_green + 1)]);
    cast_with_floating(&mut engine, P0, forgotten_lore());

    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, P0);
            assert_eq!(options, vec![P1], "the caster is not a legal target");
            assert!(engine.apply(P0, PlayerAction::ChoosePlayer(P0)).is_err());
            engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        }
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert_eq!(player, P0);
            assert!(options.is_empty(), "no graveyard card is targeted");
            assert_eq!(player_options, vec![P1]);
            assert!(
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![P0],
                        },
                    )
                    .is_err()
            );
            lk::aim(&mut engine, &[], &[P1]);
        }
        other => panic!("Forgotten Lore targets an opponent: {other:?}"),
    }
    (engine, buried)
}

#[track_caller]
fn next_question(engine: &mut Engine<RegistryLookup>) -> Pending {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::YesNo { .. }
        ) || (matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e))
    });
    engine.pending().clone()
}

#[track_caller]
fn card_choice(engine: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = next_question(engine)
    else {
        panic!("expected a graveyard choice, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (1, 1));
    (player, options)
}

#[track_caller]
fn choose(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
}

#[track_caller]
fn pay(engine: &mut Engine<RegistryLookup>, yes: bool) {
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = next_question(engine)
    else {
        panic!(
            "expected the optional green payment, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    assert_eq!(cost, baylee_core::mana!("{G}"));
    lk::yes_no(engine, yes);
}

#[test]
fn forgotten_lore_opponent_rechooses_new_cards_and_only_the_last_returns() {
    let (mut engine, buried) = cast(
        &[
            index::GRIZZLY_BEARS,
            index::HILL_GIANT,
            index::LIGHTNING_BOLT,
        ],
        2,
    );
    let (player, options) = card_choice(&mut engine);
    assert_eq!(player, P1, "the targeted opponent chooses, not the caster");
    assert_eq!(options, buried);
    choose(&mut engine, P1, buried[0]);
    pay(&mut engine, true);

    let (player, options) = card_choice(&mut engine);
    assert_eq!(player, P1);
    assert_eq!(options, buried[1..], "the first pick cannot be repeated");
    assert_eq!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS),
        Some(buried[0]),
        "the first pick stays in the graveyard while the process repeats"
    );
    choose(&mut engine, P1, buried[1]);
    pay(&mut engine, true);

    let (_, options) = card_choice(&mut engine);
    assert_eq!(options, buried[2..]);
    choose(&mut engine, P1, buried[2]);
    pay(&mut engine, false);
    lk::settle(&mut engine);

    assert_eq!(
        in_hand(&engine, P0, index::LIGHTNING_BOLT),
        Some(buried[2]),
        "only the last chosen card returns"
    );
    assert_eq!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS),
        Some(buried[0])
    );
    assert_eq!(
        in_graveyard(&engine, P0, index::HILL_GIANT),
        Some(buried[1])
    );
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 0);
}

#[test]
fn forgotten_lore_returns_the_first_choice_when_payment_is_declined() {
    let (mut engine, buried) = cast(&[index::GRIZZLY_BEARS], 1);
    assert_eq!(card_choice(&mut engine).0, P1);
    choose(&mut engine, P1, buried[0]);
    pay(&mut engine, false);
    lk::settle(&mut engine);

    assert_eq!(in_hand(&engine, P0, index::GRIZZLY_BEARS), Some(buried[0]));
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Green),
        1,
        "declining spends nothing"
    );
}

#[test]
fn forgotten_lore_without_repeat_mana_can_decline_and_return_the_last_choice() {
    let (mut engine, buried) = cast(&[index::GRIZZLY_BEARS, index::HILL_GIANT], 0);
    assert_eq!(card_choice(&mut engine).0, P1);
    choose(&mut engine, P1, buried[0]);
    pay(&mut engine, false);
    lk::settle(&mut engine);
    assert_eq!(in_hand(&engine, P0, index::GRIZZLY_BEARS), Some(buried[0]));
    assert_eq!(
        in_graveyard(&engine, P0, index::HILL_GIANT),
        Some(buried[1])
    );
}

#[test]
fn forgotten_lore_with_an_empty_graveyard_asks_for_no_card_or_payment() {
    let (mut engine, _) = cast(&[], 3);
    assert!(matches!(
        next_question(&mut engine),
        Pending::Priority { .. }
    ));
    assert!(in_graveyard(&engine, P0, forgotten_lore()).is_some());
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 3);
}

/// Paying to repeat when every card has been chosen leaves nothing to choose:
/// no empty card question is asked (the repeated "you may pay {G}" is), and
/// the last chosen card still comes back.
#[test]
fn forgotten_lore_repeat_with_no_unchosen_card_returns_the_last_choice() {
    let (mut engine, buried) = cast(&[index::GRIZZLY_BEARS], 1);
    assert_eq!(card_choice(&mut engine).0, P1);
    choose(&mut engine, P1, buried[0]);
    pay(&mut engine, true);
    pay(&mut engine, false);
    lk::settle(&mut engine);

    assert_eq!(in_hand(&engine, P0, index::GRIZZLY_BEARS), Some(buried[0]));
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 0);
}
