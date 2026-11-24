//! `cards/sorceries/mv_5/global_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn global_ruin() -> CardIndex {
    card_index("4a7b89b2-5fe9-4d39-9d42-3ca78bcf6d40")
}

fn pass_to_keep_choice(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::KeepPermanents,
                ..
            }
        )
    });
}

fn keep_choice(engine: &Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a land keep choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::KeepPermanents);
    assert_eq!((min, max), (1, 1));
    (player, options)
}

fn keep(engine: &mut Engine<RegistryLookup>, player: PlayerId, object: ObjectId) {
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .unwrap();
}

#[test]
fn global_ruin_reuses_dual_lands_and_sacrifices_only_unchosen_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                index::VOLCANIC_ISLAND,
                forest(),
                sol_ring(),
            ],
        )
        .battlefield(
            1,
            &[
                index::TROPICAL_ISLAND,
                mountain(),
                mountain(),
                index::WASTES,
                sol_ring(),
            ],
        )
        .hand(0, &[global_ruin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_plains = all_on_battlefield(&engine, p0, plains());
    let p1_mountains = all_on_battlefield(&engine, p1, mountain());
    assert_eq!(p0_plains.len(), 2);
    assert_eq!(p1_mountains.len(), 2);
    let volcanic = on_battlefield(&engine, p0, index::VOLCANIC_ISLAND).unwrap();
    let tropical = on_battlefield(&engine, p1, index::TROPICAL_ISLAND).unwrap();
    let p0_forest = on_battlefield(&engine, p0, forest()).unwrap();
    let wastes = on_battlefield(&engine, p1, index::WASTES).unwrap();
    let p0_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let p1_ring = on_battlefield(&engine, p1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, p0, global_ruin());
    pass_to_keep_choice(&mut engine);
    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p0);
    assert_eq!(options, p0_plains);
    keep(&mut engine, p0, p0_plains[0]);

    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p1);
    assert_eq!(options, p1_mountains);
    for candidate in [p0_plains[1], p1_mountains[1], wastes] {
        assert_eq!(
            engine.state().object(candidate).unwrap().zone,
            Zone::Battlefield,
            "no land is sacrificed before the last player has chosen"
        );
    }
    keep(&mut engine, p1, p1_mountains[0]);
    pass_until(&mut engine, stack_is_empty);

    for survivor in [
        p0_plains[0],
        volcanic,
        p0_forest,
        p0_ring,
        tropical,
        p1_mountains[0],
        p1_ring,
    ] {
        assert_eq!(
            engine.state().object(survivor).unwrap().zone,
            Zone::Battlefield
        );
    }
    for sacrificed in [p0_plains[1], p1_mountains[1], wastes] {
        assert_eq!(
            engine.state().object(sacrificed).unwrap().zone,
            Zone::Graveyard
        );
    }
    assert!(in_graveyard(&engine, p0, global_ruin()).is_some());
}
