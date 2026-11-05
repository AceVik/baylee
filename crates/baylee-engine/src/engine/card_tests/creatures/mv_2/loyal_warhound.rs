//! `cards/creatures/mv_2/loyal_warhound.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("cc6a83c7-e645-4a53-9550-be79b42cd851")
}

#[test]
fn loyal_warhound_fetches_a_basic_plains_tapped_when_behind_on_lands() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| {
        matches!(
            g.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let plains_card = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![plains_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let hound = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, hound).contains(KeywordSet::VIGILANCE));
    assert_eq!(
        e.state().object(plains_card).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&e, plains_card));
}

fn savannah() -> CardIndex {
    card_index("703243f0-8cb3-420f-958f-5fd4bde30293")
}

/// "A basic Plains card": Savannah, a Plains that is not basic, is not
/// offered, nor a Forest; and with lands equal nothing triggers (CR 603.4).
/// The first test's library was all basic Plains and its opponent ahead.
#[test]
fn loyal_warhound_finds_only_a_basic_plains_and_needs_an_opponent_ahead() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject(), subject(), plains(), savannah()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let basic = in_hand(&e, p0, plains()).expect("a basic Plains");
    let dual = in_hand(&e, p0, savannah()).expect("Savannah");
    for card in [basic, dual] {
        e.state
            .move_object(
                card,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| {
        matches!(
            g.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![basic], "no Savannah, no Forest");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![basic],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, basic));

    // Three lands each now: a second Warhound finds nobody ahead.
    {
        let state = e.dev_state_mut(p0).expect("dev board");
        state.players[0].mana_pool.add(ManaColor::White, 2);
    }
    e.refresh_offer();
    cast_with_floating(&mut e, p0, subject());
    pass_until(&mut e, |g| {
        stack_is_empty(g)
            || matches!(
                g.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::SearchLibrary,
                    ..
                }
            )
    });
    assert!(stack_is_empty(&e), "nothing triggered: {:?}", e.pending());
    assert_eq!(all_on_battlefield(&e, p0, subject()).len(), 2);
}
