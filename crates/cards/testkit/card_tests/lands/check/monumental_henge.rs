//! `cards/lands/check/monumental_henge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn monumental_henge_enters_untapped_with_plains_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(207, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[monumental_henge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, monumental_henge());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, monumental_henge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn henge_inspects_the_printed_count_and_reveals_only_the_matching_card() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MONUMENTAL_HENGE, plains()])
        .hand(0, &[index::MIND_STONE, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::MONUMENTAL_HENGE).unwrap();
    let chosen = hand_to_library_top(&mut e, p0, index::MIND_STONE);
    hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let s = e.dev_state_mut(p0).unwrap();
    s.set_tapped(land, false);
    s.players[0].mana_pool.add(ManaColor::White, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::MONUMENTAL_HENGE, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(options, &vec![chosen]);
    assert_eq!((*min, *max), (0, 1));
    assert_eq!(e.inspected_cards().len(), 5);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(chosen).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
}

/// Opens a Henge game whose library top five holds `top` (first listed
/// ends up deepest), floats the `{2}{W}{W}` and activates the Henge.
/// Returns the engine, the Henge and the library objects of `top`, in order.
fn henge_look(top: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MONUMENTAL_HENGE, plains()])
        .hand(0, top)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::MONUMENTAL_HENGE).unwrap();
    let on_top: Vec<ObjectId> = top
        .iter()
        .map(|card| hand_to_library_top(&mut e, p0, *card))
        .collect();
    let s = e.dev_state_mut(p0).unwrap();
    s.set_tapped(land, false);
    s.players[0].mana_pool.add(ManaColor::White, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::MONUMENTAL_HENGE, 1);
    (e, land, on_top)
}

/// "You may reveal a historic card from among them" — historic is artifact,
/// legendary or Saga. The artifact arm has its own test above; this one
/// separates the other two: a legendary creature that is no artifact and a
/// Saga that is no artifact are each takeable, a vanilla creature is not.
#[test]
fn henge_takes_a_legendary_card_or_a_saga_that_is_no_artifact_but_not_a_plain_creature() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for take_the_legend in [true, false] {
        let (mut e, land, on_top) = henge_look(&[
            index::MARHAULT_ELSDRAGON,
            index::FABLE_OF_THE_MIRROR_BREAKER,
            index::GRIZZLY_BEARS,
        ]);
        let [legend, saga, plain] = on_top[..] else {
            unreachable!()
        };
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        assert!(
            options.contains(&legend),
            "historic: a legendary card that is not an artifact"
        );
        assert!(
            options.contains(&saga),
            "historic: a Saga that is neither legendary nor an artifact"
        );
        assert!(
            !options.contains(&plain),
            "a nonlegendary, non-Saga, nonartifact creature is not historic"
        );
        assert_eq!(options.len(), 2, "exactly the two historic cards");
        assert_eq!((min, max), (0, 1), "you may reveal one historic card");
        let chosen = if take_the_legend { legend } else { saga };
        let left = if take_the_legend { saga } else { legend };
        let library = library_size(&e, p0);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(chosen).unwrap().zone,
            Zone::Hand,
            "the revealed historic card goes to hand"
        );
        assert_eq!(
            library_size(&e, p0),
            library - 1,
            "only the one card left the library"
        );
        for rest in [left, plain] {
            assert_eq!(
                e.state().object(rest).unwrap().zone,
                Zone::Library,
                "the rest go to the bottom of the library, not to hand"
            );
        }
        assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    }
}

/// With no historic card among the five nothing can be revealed: the
/// hand is unchanged and every looked-at card is still in the library.
#[test]
fn henge_with_no_historic_card_among_the_five_takes_nothing() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let (mut e, _land, on_top) = henge_look(&[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES]);
    let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = library_size(&e, p0);
    let mut spins = 0;
    while !stack_is_empty(&e) || matches!(e.pending(), Pending::ChooseCards { .. }) {
        spins += 1;
        assert!(spins < 20, "the ability never finished: {:?}", e.pending());
        if let Pending::ChooseCards { options, .. } = e.pending().clone() {
            assert!(
                options.is_empty(),
                "nothing among the five is historic: {options:?}"
            );
            e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
                .unwrap();
        } else {
            pass_once_priority(&mut e);
        }
    }
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "no historic card, nothing to hand"
    );
    assert_eq!(library_size(&e, p0), library, "the library keeps all five");
    for card in on_top {
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Library);
    }
}

fn pass_once_priority(e: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority expected, got {:?}", e.pending());
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
}
