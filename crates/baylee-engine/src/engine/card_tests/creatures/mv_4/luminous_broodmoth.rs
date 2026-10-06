//! `cards/creatures/mv_4/luminous_broodmoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Luminous Broodmoth is `Coverage::Partial`: the printed flying is on the
/// card and the second sentence — "whenever a creature you control without
/// flying dies, return it to the battlefield under its owner's control with a
/// flying counter on it" — is not.
///
/// So the Broodmoth is cast for {2}{W}{W} and lands as the 3/4 flyer it
/// prints, which is the half that is written; then a creature under the same
/// seat with no flying is destroyed through the stack, which is exactly the
/// event the missing sentence is about. Vindicate is the kill because it
/// leaves the card in a graveyard rather than exiling it, and the Broodmoth
/// standing untouched beside the dead Elf is the control: the assertion is
/// about the Elf, not about a spell that ate the wrong permanent.
#[test]
fn luminous_broodmoth_flies_in_and_leaves_a_dead_elf_in_the_graveyard() {
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

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Elf died"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and stayed in its owner's graveyard: nothing returned it with a \
         flying counter, which is the second sentence the card does not have \
         yet"
    );
    assert!(
        on_battlefield(&engine, p0, luminous_broodmoth()).is_some(),
        "while the Broodmoth stood untouched through both spells"
    );
}
