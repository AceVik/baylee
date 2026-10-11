//! `cards/lands/reveal/temple_of_the_dragon_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temple of the Dragon Queen prints `As this land enters, you may reveal a Dragon
/// card from your hand. This land enters tapped unless you revealed a Dragon card
/// this way or you control a Dragon.`, `As this land enters, choose a color.`, and
/// `{{T}}: Add one mana of the chosen color.`
///
/// With no Dragon in hand or under your control, the land enters tapped
/// and prompts for a color choice via `Pending::ChooseColor`.
/// After advancing to the next turn, the land untaps and taps for one mana of
/// the chosen color.
#[test]
fn temple_of_the_dragon_queen_enters_tapped_chooses_color_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[temple_of_the_dragon_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, temple_of_the_dragon_queen()).expect("temple in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, baylee_cards_dsl::ALL_MANA_COLORS.to_vec());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let land =
        on_battlefield(&engine, p0, temple_of_the_dragon_queen()).expect("temple on battlefield");
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, temple_of_the_dragon_queen(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn reveal_or_control_are_alternatives_and_a_second_entry_question_is_not_lost() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (land, creature, asks_color) in [
        (
            index::TEMPLE_OF_THE_DRAGON_QUEEN,
            index::SHIVAN_DRAGON,
            true,
        ),
        (index::FORTIFIED_BEACHHEAD, index::BENALISH_HERO, false),
    ] {
        for (control, reveal) in [(false, false), (false, true), (true, false), (true, true)] {
            let board = if control { vec![creature] } else { vec![] };
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &board)
                .hand(0, &[land, index::CRIB_SWAP])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let card = in_hand(&e, p0, land).unwrap();
            let show = in_hand(&e, p0, index::CRIB_SWAP).unwrap();
            let before = e.state().journal.last_seq();
            e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
            let Pending::ChooseCards {
                options, prompt, ..
            } = e.pending()
            else {
                panic!("reveal choice: {:?}", e.pending())
            };
            assert_eq!(*prompt, crate::choice::ChoicePrompt::RevealOrEnterTapped);
            assert_eq!(options, &[show]);
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: if reveal { vec![show] } else { vec![] },
                },
            )
            .unwrap();
            if asks_color {
                assert!(
                    matches!(e.pending(), Pending::ChooseColor { .. }),
                    "reveal must not consume the color choice"
                );
                e.apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
                    .unwrap();
                assert_eq!(
                    e.state().object(card).unwrap().chosen_color,
                    Some(ManaColor::Black)
                );
            }
            assert_eq!(is_tapped(&e, card), !control && !reveal);
            assert_eq!(e.state().object(show).unwrap().zone, Zone::Hand);
            let shown=e.state().journal.entries().iter().skip(before as usize).any(|entry| matches!(&entry.event,crate::event::GameEvent::Revealed{cards,..} if cards.contains(&show)));
            assert_eq!(
                shown, reveal,
                "revealing remains optional even with the subtype on board"
            );
        }
    }
}
