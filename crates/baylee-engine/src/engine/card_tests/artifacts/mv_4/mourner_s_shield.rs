//! Mourner's Shield prevents a chosen source only when its color matches imprint.

use super::*;

fn mourner_s_shield() -> CardIndex {
    card_index("34cadd9d-b8e6-422c-b0ec-a2420cd983ad")
}

#[test]
fn mourner_s_shield_prevents_matching_color_damage_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .battlefield(1, &[gray_ogre(), llanowar_elves()])
        .hand(0, &[mourner_s_shield(), lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bolt = in_hand(&e, p0, lightning_bolt()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            bolt,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    for source in all_on_battlefield(&e, p0, island()).into_iter().take(4) {
        e.apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut e, p0, mourner_s_shield());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, mourner_s_shield(), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = e.pending().clone()
    else {
        panic!("expected source choice")
    };
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert!(options.iter().any(|source| source.object == ogre));
    assert!(!options.iter().any(|source| source.object == elf));
    let source = options
        .into_iter()
        .find(|source| source.object == ogre)
        .unwrap();
    e.apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ogre, Defender::Player(p0)), (elf, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        e.state().players[0].life,
        19,
        "only the green Elf deals damage"
    );
}
