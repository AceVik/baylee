//! `cards/creatures/mv_4/luminous_broodmoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A nonflying creature returns with a real flying counter; another death
/// with that counter no longer qualifies for the Broodmoth's ability.
#[test]
fn luminous_broodmoth_returns_a_dead_elf_with_a_flying_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(89, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[luminous_broodmoth(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    // One Swamp sits out the Broodmoth's {2}{W}{W}: the Vindicate behind it
    // is {1}{W}{B}, and a payment that spent both black sources on the first
    // cast would leave the second uncastable for a reason that has nothing
    // to do with the card under test.
    let held = on_battlefield(&engine, p0, swamp()).expect("a Swamp is out");
    tap_mana_except(&mut engine, p0, held);
    let moth_card = in_hand(&engine, p0, luminous_broodmoth()).expect("the Broodmoth is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: moth_card })
        .expect("{2}{W}{W} is in the pool");
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, luminous_broodmoth()).is_some()
    });

    let moth = on_battlefield(&engine, p0, luminous_broodmoth()).expect("the Broodmoth resolved");
    assert_eq!(pt(&engine, moth), (3, 4), "the body the card prints");
    assert!(
        keywords(&engine, moth).contains(KeywordSet::FLYING),
        "and the one line of its text that is implemented"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the Elf has no flying to save it"
    );

    // The held Swamp pays the {1}{W}{B}, and the Elf is what it is pointed at.
    tap_all_mana(&mut engine, p0);
    let doom = in_hand(&engine, p0, vindicate()).expect("the Vindicate is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: doom })
        .expect("one white, one black and one generic");
    let mut aimed = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                assert!(
                    options.contains(&elves),
                    "\"destroy target permanent\" reaches a creature of your own: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![elves],
                        },
                    )
                    .expect("the Elf was one of the options");
                aimed = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected a target for the Vindicate, got {other:?}"),
        }
    }
    assert!(aimed, "the Vindicate asks what it is pointed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, p0, llanowar_elves()), Some(elves));
    assert_eq!(counters_on(&engine, elves, CounterKind::Flying), 1);
    assert!(keywords(&engine, elves).contains(KeywordSet::FLYING));
    kill(&mut engine, elves);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(
        on_battlefield(&engine, p0, luminous_broodmoth()).is_some(),
        "while the Broodmoth stood untouched through both spells"
    );
}
