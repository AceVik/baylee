//! `cards/instants/mv_1/shallow_grave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shallow_grave() -> CardIndex {
    card_index("9843d8a6-bcbf-40c2-8c41-316e97db9c62")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn gray_ogre() -> CardIndex {
    card_index("83c8a3a6-2e1a-4e26-8847-6d066f42d906")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn in_exile(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    e.state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

#[test]
fn shallow_grave_returns_top_creature_with_haste_and_exiles_at_end_step() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), grizzly_bears(), gray_ogre()])
        .hand(0, &[shallow_grave(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&e, P0, gray_ogre()).unwrap();
    // Bury Bears first, then Ogre, then put Swamp on top so Ogre is the top creature card below Swamp.
    bury(&mut e, &[bears]);
    bury(&mut e, &[ogre]);
    hand_to_graveyard(&mut e, P0, swamp());

    cast_from_hand(&mut e, P0, shallow_grave());
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, gray_ogre()).is_some(),
        "top creature returned"
    );
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "lower creature stays in graveyard"
    );
    let revived = on_battlefield(&e, P0, gray_ogre()).unwrap();
    assert!(keywords_of(&e, revived).contains(KeywordSet::HASTE));

    // Can attack immediately thanks to haste.
    kit::attack(&mut e, &[revived]);
    pass_until(&mut e, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::SecondMain
    });
    assert_eq!(e.state().players[1].life, 18);

    // Exiled at the beginning of the next end step.
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(in_exile(&e, P0, gray_ogre()));
    assert!(on_battlefield(&e, P0, gray_ogre()).is_none());
}

#[test]
fn shallow_grave_with_no_creature_in_graveyard_resolves_as_no_op() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[shallow_grave(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    hand_to_graveyard(&mut e, P0, swamp());

    cast_from_hand(&mut e, P0, shallow_grave());
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(e.state().delayed.is_empty());
    assert!(in_graveyard(&e, P0, swamp()).is_some());
}
