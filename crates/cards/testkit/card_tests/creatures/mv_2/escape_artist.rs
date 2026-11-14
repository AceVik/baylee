//! `cards/creatures/mv_2/escape_artist.rs`, played.
//!
//! Oracle:
//! - "This creature can't be blocked."
//! - "{U}, Discard a card: Return this creature to its owner's hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn escape_artist() -> CardIndex {
    card_index("74f4d505-e114-47be-be27-a7949e8abdc7")
}

#[test]
fn escape_artist_cannot_be_blocked_and_bounces_self_by_discarding() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS])
        .hand(0, &[escape_artist(), forest()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast Escape Artist using 2 Islands, leaving 1 Island + 1 Forest in hand.
    let islands = all_on_battlefield(&engine, P0, island());
    for &isl in &islands[..2] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: isl })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, escape_artist());
    pass_until(&mut engine, stack_is_empty);
    let artist = on_battlefield(&engine, P0, escape_artist()).expect("Escape Artist resolved");
    assert_eq!(pt(&engine, artist), (1, 1));
    assert!(keywords(&engine, artist).contains(KeywordSet::UNBLOCKABLE));

    // With {U} available on next own turn, test unblockable attack, then bounce.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let blockers = attack_and_collect_blocks(&mut engine, artist, P1);
    assert!(
        blockers.iter().all(|o| !o.attackers.contains(&artist)),
        "neither Air Elemental nor Grizzly Bears may block Escape Artist: {blockers:?}"
    );
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 19);

    // Activate ability 1: "{U}, Discard a card: Return this creature to its owner's hand."
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, escape_artist(), 1);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt: ChoicePrompt::CostDiscard,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostDiscard prompt, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (1, 1));
    let discard = options[0];
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, escape_artist()).is_none());
    assert!(in_hand(&engine, P0, escape_artist()).is_some());
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .is_empty()
    );
}
