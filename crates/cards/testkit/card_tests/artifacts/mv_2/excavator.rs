//! `cards/artifacts/mv_2/excavator.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn excavator() -> CardIndex {
    card_index("2d5ebcd7-07c2-412e-b0d9-54e3a88895fd")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Taiga: a land that is a Mountain and a Forest but is not basic.
fn taiga() -> CardIndex {
    card_index("22e3cf1d-3559-4ce1-954c-8dc815342979")
}

const WALKS: [KeywordSet; 5] = [
    KeywordSet::PLAINSWALK,
    KeywordSet::ISLANDWALK,
    KeywordSet::SWAMPWALK,
    KeywordSet::MOUNTAINWALK,
    KeywordSet::FORESTWALK,
];

/// p0 has an Excavator, one of each basic land, a Taiga and a Grizzly Bears;
/// p0's first main phase.
fn an_excavator_over_every_land() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                excavator(),
                plains(),
                island(),
                swamp(),
                mountain(),
                forest(),
                taiga(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Activates the Excavator on the Bears and stops on the sacrifice question,
/// returning the lands it offers.
#[track_caller]
fn to_the_sacrifice_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let bears = on_battlefield(engine, P0, grizzly_bears()).unwrap();
    activate(engine, P0, excavator(), 0);
    aim_at(engine, P0, bears);
    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        )
    };
    options
}

/// Sacrifices `land` to the Excavator aimed at the Bears and lets the ability
/// resolve; returns the Bears' keywords afterwards.
#[track_caller]
fn sacrifice_for(engine: &mut Engine<RegistryLookup>, land: CardIndex) -> KeywordSet {
    let bears = on_battlefield(engine, P0, grizzly_bears()).unwrap();
    let chosen = on_battlefield(engine, P0, land).unwrap();
    let options = to_the_sacrifice_question(engine);
    assert!(options.contains(&chosen), "the land is on the menu");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("sacrifice the land");
    pass_until(engine, stack_is_empty);
    assert!(
        on_battlefield(engine, P0, land).is_none(),
        "the land was sacrificed"
    );
    keywords(engine, bears)
}

/// Excavator (2d5ebcd7-07c2-412e-b0d9-54e3a88895fd) — {2} Artifact:
/// "{T}, Sacrifice a basic land: Target creature gains landwalk of each of the land types of the sacrificed land until end of turn."
#[test]
fn excavator_grants_the_landwalk_of_the_sacrificed_land_and_no_other() {
    let lands = [plains(), island(), swamp(), mountain(), forest()];
    for (land, walk) in lands.into_iter().zip(WALKS) {
        let mut engine = an_excavator_over_every_land();
        let granted = sacrifice_for(&mut engine, land);

        assert!(granted.contains(walk), "the sacrificed land's own walk");
        for other in WALKS.into_iter().filter(|&other| other != walk) {
            assert!(
                !granted.contains(other),
                "only the sacrificed land's type is granted, not {other:?}"
            );
        }
    }
}

#[test]
fn excavator_offers_every_basic_land_but_not_a_nonbasic_one() {
    let mut engine = an_excavator_over_every_land();
    let taiga_id = on_battlefield(&engine, P0, taiga()).unwrap();

    let options = to_the_sacrifice_question(&mut engine);

    for land in [plains(), island(), swamp(), mountain(), forest()] {
        let id = on_battlefield(&engine, P0, land).unwrap();
        assert!(options.contains(&id), "a basic land is a legal sacrifice");
    }
    assert!(
        !options.contains(&taiga_id),
        "a Taiga has basic land types but is not a basic land: {options:?}"
    );
}

#[test]
fn excavator_is_not_offered_when_the_only_land_is_nonbasic() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[excavator(), taiga(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let exc = on_battlefield(&engine, P0, excavator()).unwrap();

    assert!(
        !ability_offered(&engine, exc, 0),
        "no basic land to sacrifice, so the cost cannot be paid"
    );
}

#[test]
fn excavator_taps_and_cannot_be_used_twice_in_a_turn() {
    let mut engine = an_excavator_over_every_land();
    let exc = on_battlefield(&engine, P0, excavator()).unwrap();
    assert!(ability_offered(&engine, exc, 0));

    sacrifice_for(&mut engine, forest());

    assert!(is_tapped(&engine, exc), "{{T}} was part of the cost");
    assert!(
        !ability_offered(&engine, exc, 0),
        "a tapped Excavator pays no second {{T}}"
    );
}

#[test]
fn excavator_landwalk_ends_with_the_turn() {
    let mut engine = an_excavator_over_every_land();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert!(sacrifice_for(&mut engine, swamp()).contains(KeywordSet::SWAMPWALK));

    reach_their_main_phase(&mut engine, P1);

    let after = keywords(&engine, bears);
    for walk in WALKS {
        assert!(!after.contains(walk), "until end of turn: {walk:?} is gone");
    }
}

#[test]
fn excavator_can_aim_at_a_creature_the_opponent_controls() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[excavator(), island(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let theirs = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let land = on_battlefield(&engine, P0, island()).unwrap();

    activate(&mut engine, P0, excavator(), 0);
    let menu = aim_at(&mut engine, P0, theirs);
    assert!(menu.contains(&theirs), "\"target creature\" is anyone's");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("sacrifice the Island");
    pass_until(&mut engine, stack_is_empty);

    let granted = keywords(&engine, theirs);
    assert!(granted.contains(KeywordSet::ISLANDWALK));
    assert!(!granted.contains(KeywordSet::MOUNTAINWALK));
}
