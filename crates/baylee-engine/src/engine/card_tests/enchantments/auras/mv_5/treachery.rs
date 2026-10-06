//! `cards/enchantments/auras/mv_5/treachery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Treachery: "When this Aura enters, untap up to five lands. You control
/// enchanted creature." The lands are chosen as the trigger resolves, not
/// targeted, so the five that paid for it come back; the creature changes
/// sides for as long as the Aura stays and goes home when it leaves.
#[test]
fn treachery_steals_the_creature_and_untaps_the_lands_that_paid() {
    let treachery = card_index("8ed57194-7508-4aef-9373-64f7e80612d8");
    let balthor = card_index("23669721-fe9e-49d7-9504-ae6164de723a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(109, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[balthor])
        .hand(0, &[treachery])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lord = on_battlefield(&engine, p1, balthor).expect("Balthor is out");

    cast_from_hand(&mut engine, p0, treachery);
    aim_at(&mut engine, p0, lord);
    let tapped = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                e.state().object(**id).is_some_and(|o| {
                    o.controller == p0
                        && o.characteristics().types.contains(TypeSet::LAND)
                        && o.status.contains(crate::object::Status::TAPPED)
                })
            })
            .copied()
            .collect::<Vec<_>>()
    };
    assert_eq!(tapped(&engine).len(), 5, "the five Islands paid for it");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 5), "up to five");
    let lands = tapped(&engine);
    assert!(
        lands.iter().all(|l| options.contains(l)),
        "every tapped land is offered"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: lands })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(tapped(&engine).is_empty(), "all five untapped");
    let controller = |e: &Engine<RegistryLookup>| e.state().object(lord).unwrap().controller;
    assert_eq!(controller(&engine), p0, "you control enchanted creature");

    let aura = on_battlefield(&engine, p0, treachery).expect("Treachery is attached");
    kill(&mut engine, aura);
    assert_eq!(
        controller(&engine),
        p1,
        "with the Aura gone, Balthor goes home"
    );
}
