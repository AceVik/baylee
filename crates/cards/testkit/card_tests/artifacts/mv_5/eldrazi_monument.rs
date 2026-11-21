//! `cards/artifacts/mv_5/eldrazi_monument.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn eldrazi_monument() -> CardIndex {
    card_index("c7ee1b21-d02e-49ef-95d6-d6dbf2f886ae")
}

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Eldrazi Monument (c7ee1b21-d02e-49ef-95d6-d6dbf2f886ae) — {5} Artifact:
/// "Creatures you control get +1/+1 and have flying and indestructible."
/// "At the beginning of your upkeep, sacrifice a creature. If you can't,
/// sacrifice this artifact."
///
/// Buffs your creatures (+1/+1, flying, indestructible) and NOT opponent's;
/// indestructible survives Terror; upkeep forces sacrificing a creature even
/// though it is indestructible.
#[test]
fn eldrazi_monument_buffs_your_creatures_and_sacrifices_creature_on_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eldrazi_monument(), grizzly_bears(), hill_giant()])
        .battlefield(1, &[grizzly_bears(), swamp(), swamp()])
        .hand(1, &[terror()])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 upkeep: must sacrifice one of your creatures (choose Grizzly Bears).
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    assert_eq!(pt(&engine, giant), (4, 4), "Hill Giant gets +1/+1");
    assert!(keywords(&engine, giant).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, giant).contains(KeywordSet::INDESTRUCTIBLE));
    assert_eq!(
        pt(&engine, p1_bears),
        (2, 2),
        "opponent's creature unbuffed"
    );
    assert!(!keywords(&engine, p1_bears).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("sacrifice indestructible Grizzly Bears to Eldrazi Monument upkeep");
    reach_main_phase(&mut engine, p0);

    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "indestructible creature was still sacrificed"
    );
    assert!(
        on_battlefield(&engine, p0, eldrazi_monument()).is_some(),
        "Eldrazi Monument stays after a creature is sacrificed"
    );

    // Opponent casts Terror on our indestructible 4/4 Hill Giant -> survives!
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let ter = in_hand(&engine, p1, terror()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: ter })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, hill_giant()).is_some(),
        "indestructible Hill Giant survived Terror"
    );
}

/// If you control no creatures at your upkeep (even when opponent controls
/// creatures), Eldrazi Monument sacrifices itself.
#[test]
fn eldrazi_monument_sacrifices_itself_when_you_control_no_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eldrazi_monument()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        in_graveyard(&engine, p0, eldrazi_monument()).is_some(),
        "Eldrazi Monument sacrificed itself when p0 had no creature to sacrifice"
    );
    assert!(
        on_battlefield(&engine, PlayerId::new(1), grizzly_bears()).is_some(),
        "opponent's creature was untouched"
    );
}
