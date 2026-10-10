//! `cards/creatures/mv_4/serendib_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn serendib_djinn() -> CardIndex {
    card_index("683e7135-de54-49c8-a978-4f84628a6a91")
}

fn life(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

/// Passes priority until a land choice is asked or the game leaves the
/// upkeep without asking one.
fn to_land_choice(engine: &mut Engine<RegistryLookup>) -> Option<(PlayerId, Vec<ObjectId>)> {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => return Some((player, options)),
            Pending::Priority { player, .. } => {
                if !matches!(engine.state().turn.phase, Phase::Beginning) {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("never settled");
}

fn djinn_duel(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut battlefield = vec![serendib_djinn()];
    battlefield.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &battlefield)
        .battlefield(1, theirs)
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn damage_from(engine: &Engine<RegistryLookup>, source: ObjectId) -> u32 {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            crate::event::GameEvent::DamageDealt {
                source: Some(s),
                amount,
                ..
            } if s == source => Some(amount),
            _ => None,
        })
        .sum()
}

/// Serendib Djinn: choosing the Island sacrifices it and the Djinn deals 3 to
/// its controller.
#[test]
fn serendib_djinn_island_sacrificed_hurts_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = djinn_duel(&[island(), forest()], &[]);
    let djinn = on_battlefield(&engine, p0, serendib_djinn()).expect("the Djinn is out");
    let isle = on_battlefield(&engine, p0, island()).expect("the Island");
    let (who, options) = to_land_choice(&mut engine).expect("the upkeep asks");
    assert_eq!(who, p0);
    assert!(options.contains(&isle));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![isle],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(in_graveyard(&engine, p0, island()).is_some());
    assert!(on_battlefield(&engine, p0, island()).is_none());
    assert_eq!(life(&engine, 0), 17, "3 damage");
    assert_eq!(damage_from(&engine, djinn), 3, "the Djinn dealt it");
    assert!(on_battlefield(&engine, p0, serendib_djinn()).is_some());
}

/// Choosing the Forest costs no life.
#[test]
fn serendib_djinn_forest_sacrificed_costs_no_life() {
    let p0 = PlayerId::new(0);
    let mut engine = djinn_duel(&[island(), forest()], &[]);
    let djinn = on_battlefield(&engine, p0, serendib_djinn()).expect("the Djinn is out");
    let (_, options) = to_land_choice(&mut engine).expect("the upkeep asks");
    let wood = on_battlefield(&engine, p0, forest()).expect("a Forest");
    assert!(options.contains(&wood));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wood],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, island()).is_some());
    assert_eq!(life(&engine, 0), 20);
    assert_eq!(damage_from(&engine, djinn), 0);
}

/// Only the controller's lands are offered, exactly one of them is taken.
#[test]
fn serendib_djinn_offers_only_its_controllers_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = djinn_duel(&[island(), forest()], &[island(), forest()]);
    let (who, options) = to_land_choice(&mut engine).expect("the upkeep asks");
    assert_eq!(who, p0);
    for &o in &options {
        assert_eq!(engine.state().object(o).unwrap().controller, p0);
    }
    assert_eq!(options.len(), 2, "my Island and Forest: {options:?}");
    let theirs = on_battlefield(&engine, PlayerId::new(1), island()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs]
                }
            )
            .is_err()
    );
}

/// With no lands the upkeep trigger asks nothing and the state trigger
/// sacrifices the Djinn.
#[test]
fn serendib_djinn_is_sacrificed_when_its_controller_has_no_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[serendib_djinn()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(to_land_choice(&mut engine).is_none(), "nothing to choose");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(on_battlefield(&engine, p0, serendib_djinn()).is_none());
    assert!(in_graveyard(&engine, p0, serendib_djinn()).is_some());
    assert_eq!(life(&engine, 0), 20);
}

/// The opponent's upkeep does not trigger it.
#[test]
fn serendib_djinn_ignores_the_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = djinn_duel(&[island(), forest(), forest()], &[]);
    let (_, options) = to_land_choice(&mut engine).expect("my upkeep asks");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    let lands = all_on_battlefield(&engine, p0, island()).len()
        + all_on_battlefield(&engine, p0, forest()).len();
    assert_eq!(lands, 2, "only the one sacrifice, at my own upkeep");
}
