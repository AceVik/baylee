//! `cards/creatures/mv_4/magnivore.rs`, played.
//!
//! Oracle:
//! - "Haste"
//! - "Magnivore's power and toughness are each equal to the number of sorcery
//!   cards in all graveyards."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn magnivore() -> CardIndex {
    card_index("882da1c6-7b86-4de0-9161-ce7c50f9c90c")
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
fn magnivore_counts_sorceries_in_all_graveyards_and_has_haste() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                magnivore(),
                index::VOLCANIC_HAMMER,
                index::DIVINATION,
                index::SHOCK,
                bojuka_bog(),
            ],
        )
        .hand(1, &[index::VOLCANIC_HAMMER, index::GRIZZLY_BEARS])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 1 sorcery in P0's yard + 1 sorcery in P1's yard; instant & creature in yards do not count.
    hand_to_graveyard(&mut engine, P0, index::DIVINATION);
    hand_to_graveyard(&mut engine, P0, index::SHOCK);
    hand_to_graveyard(&mut engine, P1, index::VOLCANIC_HAMMER);
    hand_to_graveyard(&mut engine, P1, index::GRIZZLY_BEARS);
    let mag_hand = in_hand(&engine, P0, magnivore()).unwrap();
    assert_eq!(cda_pt(&engine, mag_hand), (2, 2));

    // Cast Magnivore with 4 Mountains -> 2/2 with Haste, attacks same turn for 2!
    let mountains = all_on_battlefield(&engine, P0, mountain());
    for &m in &mountains[..4] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, magnivore());
    pass_until(&mut engine, stack_is_empty);

    let mag = on_battlefield(&engine, P0, magnivore()).unwrap();
    assert_eq!(pt(&engine, mag), (2, 2));
    assert!(keywords(&engine, mag).contains(KeywordSet::HASTE));

    attack_and_collect_blocks(&mut engine, mag, P1);
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);

    // Cast Volcanic Hammer (sorcery) in SecondMain -> 3rd sorcery in graveyard -> Magnivore becomes 3/3.
    for &m in &mountains[4..] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, index::VOLCANIC_HAMMER);
    aim_player(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(pt(&engine, mag), (3, 3));

    // Exiling P1's graveyard removes 1 sorcery -> shrinks 3/3 -> 2/2.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    play_land(&mut engine, P0, bojuka_bog());
    aim_player(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mag), (2, 2));
}
