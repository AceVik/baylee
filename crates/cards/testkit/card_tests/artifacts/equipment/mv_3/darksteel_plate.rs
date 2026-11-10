//! `cards/artifacts/equipment/mv_3/darksteel_plate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darksteel_plate() -> CardIndex {
    card_index("b5b4cf54-ed5e-42d0-9d98-5fec76b0b0b8")
}

/// Darksteel Plate: "Indestructible. Equipped creature has indestructible.
/// Equip {2}". Destruction is the test: neither the Plate nor the creature
/// it holds goes to the graveyard, where the bystander does.
#[test]
fn darksteel_plate_is_indestructible_itself() {
    let mut r = rig(darksteel_plate(), llanowar_elves(), 2);
    assert!(keywords(&r.engine, r.equipment).contains(KeywordSet::INDESTRUCTIBLE));
    let plate = r.equipment;
    kill(&mut r.engine, plate);
    assert!(
        on_battlefield(&r.engine, PlayerId::new(0), darksteel_plate()).is_some(),
        "destroying the Plate does nothing"
    );
    let bystander = r.bystander;
    kill(&mut r.engine, bystander);
    assert!(
        on_battlefield(&r.engine, PlayerId::new(0), goblin_piker()).is_none(),
        "while an ordinary creature dies to the same destruction"
    );
}

#[test]
fn darksteel_plate_makes_the_equipped_creature_indestructible() {
    let mut r = rig(darksteel_plate(), llanowar_elves(), 2);
    assert!(!keywords(&r.engine, r.host).contains(KeywordSet::INDESTRUCTIBLE));
    r.equip(2);
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&r.engine, r.bystander).contains(KeywordSet::INDESTRUCTIBLE));
    let host = r.host;
    kill(&mut r.engine, host);
    assert!(
        on_battlefield(&r.engine, PlayerId::new(0), llanowar_elves()).is_some(),
        "the equipped Elf survives destruction"
    );
}
