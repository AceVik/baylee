//! Commander replacements retain the operation's selected cards and follow-up.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{event::Cause, zone::ZonePosition};

fn setup(
    hand: &[CardIndex],
    battlefield: &[CardIndex],
    commanders: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, battlefield)
        .commander(0, commanders)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for color in ManaColor::ALL {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
    e
}

#[test]
fn batch43_drafna_collects_all_commander_replacements_before_moving_ordered_cards() {
    for redirect in [[false, false], [true, false], [false, true], [true, true]] {
        let mut e = setup(
            &[index::DRAFNA_S_RESTORATION],
            &[],
            &[index::SOL_RING, index::MANA_VAULT],
        );
        let cards: Vec<_> = e.state().commanders[0]
            .iter()
            .map(|commander| commander.object)
            .collect();
        let state = e.dev_state_mut(P0).unwrap();
        for (at, &card) in cards.iter().enumerate() {
            state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            // Fixture: both owners already declined their graveyard SBA.
            state.commanders[0][at].answered = state.object(card).unwrap().timestamp;
        }
        e.refresh_offer();
        cast_with_floating(&mut e, P0, index::DRAFNA_S_RESTORATION);
        e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
        target(&mut e, cards.clone(), vec![]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        e.apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
        for (card, home) in cards.iter().zip(redirect) {
            assert!(matches!(e.pending(), Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::CommanderReplace { card: asked, to_library: true }, ..
            } if asked == card));
            assert!(
                cards
                    .iter()
                    .all(|card| e.state().object(*card).unwrap().zone == Zone::Graveyard)
            );
            yes(&mut e, home);
        }
        pass_until(&mut e, stack_is_empty);
        let top: Vec<_> = library(&e, P0).into_iter().rev().take(2).collect();
        let expected: Vec<_> = cards
            .iter()
            .zip(redirect)
            .filter_map(|(&card, home)| (!home).then_some(card))
            .collect();
        assert_eq!(&top[..expected.len()], expected);
        for (card, home) in cards.iter().zip(redirect) {
            assert_eq!(
                e.state().object(*card).unwrap().zone,
                if home { Zone::Command } else { Zone::Library }
            );
        }
    }
}

#[test]
fn batch43_sidequest_commander_redirect_does_not_create_food_or_transform() {
    for redirect in [false, true] {
        let mut e = setup(
            &[index::SIDEQUEST_CATCH_A_FISH],
            &[],
            &[index::GRIZZLY_BEARS],
        );
        cast_with_floating(&mut e, P0, index::SIDEQUEST_CATCH_A_FISH);
        pass_until(&mut e, stack_is_empty);
        let card = e.state().commanders[0][0].object;
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let source = on_battlefield(&e, P0, index::SIDEQUEST_CATCH_A_FISH).unwrap();
        next_upkeep(&mut e);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::PutIntoHand,
                    ..
                }
            )
        });
        let before = e.state().battlefield_seen().count();
        choose(&mut e, vec![card]);
        assert!(matches!(e.pending(), Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::CommanderReplace { card: asked, to_library: false }, ..
        } if *asked == card));
        yes(&mut e, redirect);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if redirect { Zone::Command } else { Zone::Hand }
        );
        assert_eq!(
            e.state().object(source).unwrap().face_index,
            u8::from(!redirect)
        );
        assert_eq!(
            e.state().battlefield_seen().count(),
            before + usize::from(!redirect)
        );
    }
}
