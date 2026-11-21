//! `cards/artifacts/mv_3/wirefly_hive.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_engine::event::GameEvent;

fn wirefly_hive() -> CardIndex {
    card_index("846bedc3-a1e0-4f11-a095-85b60ad482da")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wireflies_on_board(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| engine.state().names.get(o.characteristics().name) == "Wirefly")
        })
        .collect()
}

fn latest_flip(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .rev()
        .find_map(|e| match e.event {
            GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .expect("coin flip recorded")
}

fn advance_to_next_p0_main(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let start_turn = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == p0
            && engine.state().turn.phase == Phase::FirstMain
            && engine.pending().asked() == Some(p0)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine.state().zones.list(ZoneLocation::Hand(player));
                let cards: Vec<ObjectId> = hand.iter().take(usize::from(count)).copied().collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: cards })
                    .unwrap();
            }
            other => panic!("unexpected while advancing turn: {other:?}"),
        }
    }
    panic!("never reached next p0 main phase");
}

/// Wirefly Hive (846bedc3-a1e0-4f11-a095-85b60ad482da) — {3} Artifact:
/// "{3}, {T}: Flip a coin. If you win the flip, create a 2/2 colorless Insect
///  artifact creature token with flying named Wirefly. If you lose the flip,
///  destroy all permanents named Wirefly."
///
/// Activates across turns until a win creates a 2/2 colorless Insect artifact
/// creature token with flying named Wirefly, then continues until a loss
/// destroys all Wirefly permanents while sparing other creatures.
#[test]
fn wirefly_hive_creates_wirefly_on_win_and_destroys_wireflies_on_loss() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                wirefly_hive(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let mut saw_win = false;
    let mut saw_loss_wiping_wireflies = false;

    for turn in 0..20 {
        if turn > 0 {
            advance_to_next_p0_main(&mut engine);
        }
        let wireflies_before = wireflies_on_board(&engine).len();
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, wirefly_hive(), 0);
        pass_until(&mut engine, stack_is_empty);

        if latest_flip(&engine) {
            let flies = wireflies_on_board(&engine);
            assert_eq!(flies.len(), wireflies_before + 1, "win creates a Wirefly");
            let fly = engine.state().object(flies[0]).unwrap();
            assert_eq!(pt(&engine, flies[0]), (2, 2));
            assert!(fly.characteristics().colors.is_empty(), "colorless");
            assert!(fly.characteristics().types.contains(
                baylee_core::types::TypeSet::ARTIFACT.union(baylee_core::types::TypeSet::CREATURE)
            ));
            assert!(
                fly.characteristics()
                    .subtypes
                    .contains(baylee_core::generated::subtypes::creature::INSECT)
            );
            assert!(keywords(&engine, flies[0]).contains(KeywordSet::FLYING));
            saw_win = true;
        } else {
            assert!(
                wireflies_on_board(&engine).is_empty(),
                "loss destroys all permanents named Wirefly"
            );
            assert_eq!(
                engine.state().object(bears).unwrap().zone,
                Zone::Battlefield,
                "non-Wirefly creatures survive a lost flip"
            );
            if wireflies_before > 0 {
                saw_loss_wiping_wireflies = true;
                break;
            }
        }
    }

    assert!(saw_win, "must exercise winning a flip");
    assert!(
        saw_loss_wiping_wireflies,
        "must exercise losing a flip while Wireflies are on the battlefield"
    );
}
