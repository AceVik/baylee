//! `cards/enchantments/mv_3/gaea_s_anthem.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn gaeas_anthem() -> CardIndex {
    card_index("3754dce0-3e97-406f-8807-a4942a222c41")
}

// Checklist: every creature controlled by the Anthem's controller, including
// a token, gets +1/+1; opposing creatures do not; the effect ends with its source.
#[test]
fn gaeas_anthem_pumps_all_and_only_its_controllers_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[gaeas_anthem(), index::SPROUT, index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    tap_all_mana_but(&mut e, p0, Some(llanowar_elves()));
    cast_with_floating(&mut e, p0, gaeas_anthem());
    pass_until(&mut e, stack_is_empty);
    cast_from_hand(&mut e, p0, index::SPROUT);
    pass_until(&mut e, stack_is_empty);

    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("our Elf");
    let their_elf = on_battlefield(&e, p1, llanowar_elves()).expect("opposing Elf");
    let token = tokens_of(&e, p0).into_iter().next().expect("Sprout token");
    assert_eq!(pt(&e, elf), (2, 2));
    assert_eq!(pt(&e, token), (2, 2), "tokens are creatures too");
    assert_eq!(
        pt(&e, their_elf),
        (1, 1),
        "opponent's creature is unchanged"
    );

    let anthem = on_battlefield(&e, p0, gaeas_anthem()).expect("the source remains");
    cast_from_hand(&mut e, p0, index::DISENCHANT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!("Disenchant chooses its enchantment target")
    };
    assert!(options.contains(&anthem));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![anthem],
        },
    )
    .expect("Gaea's Anthem is a legal enchantment target");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, elf), (1, 1));
    assert_eq!(pt(&e, token), (1, 1));
    assert_eq!(pt(&e, their_elf), (1, 1));
}
