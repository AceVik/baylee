//! `cards/artifacts/equipment/mv_1/masterwork_of_ingenuity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn masterwork_of_ingenuity() -> CardIndex {
    card_index("35d458fb-dfdb-41e4-92e3-f0fd392e1c69")
}

fn bonesplitter() -> CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

/// Casts the Masterwork and stops at the copy question.
#[track_caller]
fn cast_to_the_copy_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    float(engine, 1);
    cast_with_floating(engine, PlayerId::new(0), masterwork_of_ingenuity());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 1), "\"you may\": choosing none is allowed");
    options
}

/// Masterwork of Ingenuity — "You may have this Equipment enter as a copy of
/// any Equipment on the battlefield." Copying a Bonesplitter: the Masterwork
/// offers only the Bonesplitter (no other permanent is an Equipment), then
/// carries Bonesplitter's Equip {1} and +2/+0.
#[test]
fn masterwork_of_ingenuity_enters_as_a_copy_of_an_equipment() {
    let elf = llanowar_elves();
    let mut engine = rig(3, &[masterwork_of_ingenuity()], &[bonesplitter(), elf], &[]);
    let p0 = PlayerId::new(0);
    let original = on_battlefield(&engine, p0, bonesplitter()).expect("the Bonesplitter");
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");

    let options = cast_to_the_copy_question(&mut engine);
    assert_eq!(options, vec![original], "the only Equipment on the table");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![original],
            },
        )
        .expect("copy the Bonesplitter");
    pass_until(&mut engine, stack_is_empty);

    let copy = on_battlefield(&engine, p0, masterwork_of_ingenuity()).expect("it entered");
    assert_ne!(copy, original);
    equip(&mut engine, masterwork_of_ingenuity(), 1, host);
    assert_eq!(pt(&engine, host), (3, 1), "a Bonesplitter's +2/+0");
    assert!(
        engine
            .state()
            .object(original)
            .is_some_and(|o| o.attached_to.is_none()),
        "the real Bonesplitter was not the one equipped"
    );
}

/// The copy is optional: declining leaves the plain Masterwork, which prints
/// no ability, so there is no Equip to offer even with mana to spare.
#[test]
fn masterwork_of_ingenuity_may_stay_itself() {
    let elf = llanowar_elves();
    let mut engine = rig(3, &[masterwork_of_ingenuity()], &[bonesplitter(), elf], &[]);
    let p0 = PlayerId::new(0);
    cast_to_the_copy_question(&mut engine);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline");
    pass_until(&mut engine, stack_is_empty);

    let plain = on_battlefield(&engine, p0, masterwork_of_ingenuity()).expect("it entered");
    float(&mut engine, 2);
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|(src, _)| *src == plain),
        "an uncopied Masterwork has no abilities"
    );
}
