//! `cards/creatures/mv_4/rooting_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn rooting_kavu() -> CardIndex {
    card_index("e7438616-cc42-4a6d-a7e3-5856cb0782fa")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|p| p.index == card))
        })
}

/// Rooting Kavu ({2}{G}{G} 4/3):
/// "When this creature dies, you may exile it. If you do, shuffle all creature
/// cards from your graveyard into your library."
#[test]
fn rooting_kavu_exiles_self_on_death_and_shuffles_creature_cards_only() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[rooting_kavu(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    // Cast Giant Growth on Bears so a noncreature card sits in graveyard; then bury Bears and Kavu.
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, giant_growth());
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Next turn cast Rooting Kavu and test dying + exiling + shuffling only creatures.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, rooting_kavu());
    pass_until(&mut engine, stack_is_empty);

    let kavu = on_battlefield(&engine, P0, rooting_kavu()).unwrap();
    bury(&mut engine, &[bears]);
    let lib_before = library_size(&engine, P0);

    bury(&mut engine, &[kavu]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, rooting_kavu()).is_none(),
        "exiled"
    );
    assert!(in_exile(&engine, P0, rooting_kavu()));
    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_none(),
        "creature card shuffled into library"
    );
    assert!(
        in_graveyard(&engine, P0, giant_growth()).is_some(),
        "noncreature card stays in graveyard"
    );
    assert_eq!(library_size(&engine, P0), lib_before + 1);
}
