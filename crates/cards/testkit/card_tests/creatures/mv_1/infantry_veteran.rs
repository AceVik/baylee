//! `cards/creatures/mv_1/infantry_veteran.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Infantry Veteran prints one line: "{T}: Target attacking creature gets
/// +1/+1 until end of turn." Two words in it have to be read off the board
/// rather than off the card file. "Attacking" is struck by attacking with one
/// Elf and leaving a second at home: both are creatures you control, and only
/// the one that turned sideways is on the menu, the Veteran itself and the Elf
/// across the table least of all. "Until end of turn" is read as a body, the
/// way `+1/-1` would be told from `+1/+1`: a +1/+0 reading lands `(2, 1)` and
/// a pump that missed the target leaves it a printed `(1, 1)`. The tap is the
/// third clause, and it is paid *after* the target (CR 601.2c, then 601.2h),
/// so the Veteran is still standing while the question is open and down the
/// moment the answer lands.
#[test]
fn infantry_veteran_pumps_only_the_creature_that_attacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                infantry_veteran(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A creature across the table that is a creature and not an attacker,
        // so "the one that attacked" is a reading and not a count of bodies.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let veteran = on_battlefield(&engine, p0, infantry_veteran()).expect("the Veteran is out");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "one Elf attacks and one stays home");
    let (attacker, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, attacker),
        (1, 1),
        "a printed 1/1 before the pump"
    );
    assert!(!is_tapped(&engine, veteran), "and the Veteran is untapped");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    // The whole price is the Veteran's own {T}, so nothing has to be floated
    // to read the offer — and `tap_all_mana` is deliberately not called, since
    // the one permanent that must stay untapped is the one it would spend.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    activate(&mut engine, p0, infantry_veteran(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&attacker),
        "the Elf that attacked is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "\"target attacking creature\": the Elf that stayed home is not one: {options:?}"
    );
    assert!(
        !options.contains(&veteran),
        "nor is the Veteran itself, which declared no attack: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "nor the Elf across the table, which attacked nothing: {options:?}"
    );
    assert_eq!(options.len(), 1, "and the attacker is the whole menu");

    assert!(
        !is_tapped(&engine, veteran),
        "the target is named before the cost is paid: still untapped while the \
         question stands"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the attacker the question offered");
    assert!(
        is_tapped(&engine, veteran),
        "and {{T}} is how the pump is paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability, so it waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, attacker),
        (2, 2),
        "a 1/1 with +1/+1 until end of turn — a (2, 1) would be +1/+0 and a \
         (1, 1) a pump that never applied"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "nor across the table");
}
