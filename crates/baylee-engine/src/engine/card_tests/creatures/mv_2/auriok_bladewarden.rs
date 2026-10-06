//! `cards/creatures/mv_2/auriok_bladewarden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Auriok Bladewarden: "{T}: Target creature gets +X/+X until end of turn, where X is this creature's power."
/// Starting as a 1/1 on the battlefield, its activated ability targets another 1/1 creature.
/// Upon resolution, the target creature receives +1/+1 based on the Bladewarden's power and becomes a 2/2.
#[test]
fn auriok_bladewarden_pumps_target_creature_by_its_own_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[auriok_bladewarden(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let warden = on_battlefield(&engine, p0, auriok_bladewarden()).expect("Bladewarden deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    activate(&mut engine, p0, auriok_bladewarden(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (2, 2), "Elves received +1/+1");
    assert!(is_tapped(&engine, warden), "Bladewarden tapped to pay cost");
}

// Aven Brigadier — {3}{W}{W}{W} — Creature — Bird Soldier 3/5 with flying.
// It prints two statics: "Other Bird creatures get +1/+1" and "Other Soldier
// creatures get +1/+1". The word that carries the card is "other", and the
// word it does *not* print is "you control" — so one board with a Bird and a
// Soldier of mine, a Bird across the table and an Elf that is neither reads
// every sentence at once, while the Brigadier itself, a Bird Soldier, is an
// "other" creature to both of its own abilities and must stay a printed 3/5.

#[test]
fn aven_brigadier_pumps_other_birds_and_soldiers_and_never_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut field = vec![plains(); 6];
    field.extend([umara_raptor(), auriok_bladewarden(), quiet_creature()]);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &field)
        // A Bird and a creature that is neither, on the far side of the
        // table: the printed sentences name no controller.
        .battlefield(1, &[umara_raptor(), quiet_creature()])
        .hand(0, &[aven_brigadier()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("my Bird is out");
    let soldier = on_battlefield(&engine, p0, auriok_bladewarden()).expect("my Soldier is out");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let their_bird = on_battlefield(&engine, p1, umara_raptor()).expect("their Bird is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, bird), (1, 1), "nothing has pumped the Bird yet");
    assert_eq!(
        pt(&engine, soldier),
        (1, 1),
        "nothing has pumped the Soldier yet"
    );
    assert_eq!(
        pt(&engine, their_bird),
        (1, 1),
        "and nothing has pumped the Bird across the table either"
    );

    // Six Plains pay {3}{W}{W}{W}; `cast_from_hand` taps the board first,
    // because the offer is read off the pool and not off the untapped lands.
    cast_from_hand(&mut engine, p0, aven_brigadier());
    pass_until(&mut engine, stack_is_empty);
    let brigadier = on_battlefield(&engine, p0, aven_brigadier()).expect("the Brigadier resolved");

    assert_eq!(
        pt(&engine, brigadier),
        (3, 5),
        "the Brigadier is itself a Bird and a Soldier, so a missing \
         Filter::Another would leave a 5/7 standing here"
    );
    assert!(
        keywords(&engine, brigadier).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        pt(&engine, bird),
        (2, 2),
        "\"Other Bird creatures get +1/+1\""
    );
    assert_eq!(
        pt(&engine, soldier),
        (2, 2),
        "\"Other Soldier creatures get +1/+1\""
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "an Elf is neither a Bird nor a Soldier: a static that had lost its \
         subtype filter would have pumped this one as well"
    );
    assert_eq!(
        pt(&engine, their_bird),
        (2, 2),
        "the sentences name no controller, so an opponent's Bird is an other \
         Bird creature — a filter carrying ControlledByYou would read wrong"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "and the Elf across the table stays the 1/1 it was printed as"
    );
}
