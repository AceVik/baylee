//! `cards/enchantments/auras/mv_2/snow_devil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn snow_devil() -> CardIndex {
    card_index("e743b4ec-65bf-4e8d-ad87-6746a2e6ba73")
}

fn snow_covered_island() -> CardIndex {
    card_index("5b2460a5-6ae5-4cad-ba94-1a9e98e6e4c0")
}

fn enchanted_bears(first_land: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[first_land, island(), grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[snow_devil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, snow_devil());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    (engine, bears)
}

/// Snow Devil (e743b4ec-65bf-4e8d-ad87-6746a2e6ba73) — {1}{U} Enchantment — Aura:
/// - "Enchant creature"
/// - "Enchanted creature has flying."
/// - "Enchanted creature has first strike as long as it's blocking and you control a snow land."
#[test]
fn snow_devil_grants_flying_always_and_first_strike_only_while_blocking_with_a_snow_land() {
    // 1. Without a snow land: enchanted creature has flying, but does NOT gain first strike even when blocking.
    let (mut engine, bears) = enchanted_bears(island());

    assert!(keywords(&engine, bears).contains(KeywordSet::FLYING));
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "not blocking and no snow land: no first strike"
    );

    // P1 attacks with 2/2 Gray Ogre; P0 blocks with 2/2 Grizzly Bears (no snow land -> no first strike -> both trade).
    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ogre)],
            },
        )
        .unwrap();
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "blocking without a snow land does not grant first strike"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none()
            && on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "without first strike, the two 2/2s trade in combat"
    );

    // 2. With a Snow-Covered Island: enchanted creature has flying, lacks first strike outside blocking,
    // and gains first strike while blocking (killing the 2/2 attacker unscathed).
    let (mut engine, bears) = enchanted_bears(snow_covered_island());

    assert!(keywords(&engine, bears).contains(KeywordSet::FLYING));
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "even with a snow land, enchanted creature does not have first strike when not blocking"
    );

    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ogre)],
            },
        )
        .unwrap();
    assert!(
        keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "blocking while controlling a snow land grants first strike"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some()
            && on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "with first strike, Grizzly Bears kills Gray Ogre before normal combat damage"
    );
}
