//! `cards/creatures/mv_8/cognivore.rs`, played.
//!
//! Oracle:
//! - "Flying"
//! - "Cognivore's power and toughness are each equal to the number of instant
//!   cards in all graveyards."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cognivore() -> CardIndex {
    card_index("8b4aae48-be01-4b42-8d32-45071650da03")
}

fn cda_pt(engine: &Engine<RegistryLookup>, id: ObjectId) -> (i16, i16) {
    let chars =
        baylee_engine::layers::recompute(engine.state(), engine.state().object(id).unwrap())
            .characteristics;
    (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
}

fn aim_player(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

#[test]
fn cognivore_counts_instants_in_all_graveyards_and_flies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::PHANTOM_MONSTER])
        .hand(
            0,
            &[
                cognivore(),
                lightning_bolt(),
                index::SHOCK,
                index::DIVINATION,
                bojuka_bog(),
            ],
        )
        .hand(1, &[index::UNSUMMON, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 1 instant in P0's yard + 1 instant in P1's yard; sorcery (Divination) and creature do NOT count.
    hand_to_graveyard(&mut engine, P0, index::SHOCK);
    hand_to_graveyard(&mut engine, P0, index::DIVINATION);
    hand_to_graveyard(&mut engine, P1, index::UNSUMMON);
    hand_to_graveyard(&mut engine, P1, index::GRIZZLY_BEARS);
    let cog_hand = in_hand(&engine, P0, cognivore()).unwrap();
    assert_eq!(cda_pt(&engine, cog_hand), (2, 2));

    // Cast Cognivore using 8 Islands, leaving Mountain untapped for Bolt.
    for isl in all_on_battlefield(&engine, P0, island()) {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: isl })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, cognivore());
    pass_until(&mut engine, stack_is_empty);

    let cog = on_battlefield(&engine, P0, cognivore()).expect("Cognivore resolved");
    assert_eq!(pt(&engine, cog), (2, 2));
    assert!(keywords(&engine, cog).contains(KeywordSet::FLYING));

    // Cast Lightning Bolt at P1 -> 3rd instant hits graveyard, Cognivore grows to 3/3.
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .unwrap();
    cast_with_floating(&mut engine, P0, lightning_bolt());
    aim_player(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, cog), (3, 3));

    // Flying: Phantom Monster (flying) can block, Grizzly Bears (ground) cannot.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let monster = on_battlefield(&engine, P1, index::PHANTOM_MONSTER).unwrap();
    let blockers = attack_and_collect_blocks(&mut engine, cog, P1);
    assert!(blockers.iter().any(|o| o.blocker == monster));
    assert!(blockers.iter().all(|o| o.blocker != bears));
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Exiling P1's graveyard removes 1 instant -> shrinks 3/3 -> 2/2.
    play_land(&mut engine, P0, bojuka_bog());
    aim_player(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, cog), (2, 2));
}
