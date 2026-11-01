//! `cards/sorceries/mv_7/sunscour.rs`, played for its pitch cost.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("155f14dd-1516-4b9f-ae39-0a6172c69a66")
}
fn white_card() -> CardIndex {
    card_index("d646e42b-5635-4798-b633-29c093b66a55")
}

#[test]
fn sunscour_exiles_exactly_two_white_cards_and_destroys_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(
            0,
            &[subject(), white_card(), serra_angel(), lightning_bolt()],
        )
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[oboro_envoy()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let spell = in_hand(&e, p0, subject()).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let mut pitch_cards = [white_card(), serra_angel()].into_iter();
    for _ in 0..8 {
        match e.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let mode = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .unwrap();
                e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
            }
            Pending::ChooseCards { options, max, .. } => {
                assert_eq!(max, 1);
                assert!(options.iter().all(
                    |id| e.state().object(*id).unwrap().card.unwrap().index != lightning_bolt()
                ));
                let card = pitch_cards.next().unwrap();
                let white = *options
                    .iter()
                    .find(|id| e.state().object(**id).unwrap().card.unwrap().index == card)
                    .unwrap();
                e.apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![white],
                    },
                )
                .unwrap();
            }
            _ => break,
        }
    }
    assert!(
        pitch_cards.next().is_none(),
        "both white cards were pitched"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(exiled(&e, p0, white_card()) >= 1);
    assert!(exiled(&e, p0, serra_angel()) >= 1);
    assert!(in_hand(&e, p0, lightning_bolt()).is_some());
    assert!(on_battlefield(&e, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&e, p1, oboro_envoy()).is_none());
}
