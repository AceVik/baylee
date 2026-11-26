//! `cards/artifacts/mv_4/hornet_cannon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
fn toughness_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<i16> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.characteristics().toughness)
}

const P1: PlayerId = PlayerId::new(1);

fn hornet_cannon() -> CardIndex {
    card_index("595ae6f1-e66a-4774-943a-9d5368f4a3b4")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn hornet_token(e: &Engine<RegistryLookup>) -> Option<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            e.state().object(id).is_some_and(|o| {
                o.card.is_none() && e.state().names.get(o.characteristics().name) == "Hornet"
            })
        })
}

#[test]
fn hornet_cannon_creates_1_1_flying_haste_artifact_insect_destroyed_at_end_step() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[hornet_cannon()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, hornet_cannon());
    pass_until(&mut e, stack_is_empty);

    activate(&mut e, P0, hornet_cannon(), 0);
    pass_until(&mut e, stack_is_empty);

    let token = hornet_token(&e).expect("Hornet token created");
    let chars = e.state().object(token).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert_eq!(
        (power_of(&e, token), toughness_of(&e, token)),
        (Some(1), Some(1))
    );
    assert!(keywords_of(&e, token).contains(KeywordSet::FLYING.union(KeywordSet::HASTE)));

    // Attacks immediately thanks to haste.
    kit::attack(&mut e, &[token]);
    pass_until(&mut e, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::SecondMain
    });
    assert_eq!(e.state().players[1].life, 19);

    // Destroyed at end step.
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(hornet_token(&e).is_none());
}
