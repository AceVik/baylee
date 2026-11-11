//! `cards/artifacts/equipment/mv_2/thornbite_staff.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thornbite_staff() -> CardIndex {
    card_index("dae4815e-9025-4993-ab46-52a3f1a7219e")
}

fn orochi_leafcaller() -> CardIndex {
    card_index("081db161-1ad0-4ee7-9d34-c415984080f0")
}

/// Equip {4} gives only its wearer the {2}, {T} ping and the dies-trigger.
/// Each lethal ping really untaps the wearer, allowing the second activation;
/// moving the Staff moves the granted activated ability with it.
#[test]
fn staff_pays_for_two_lethal_pings_untapping_between_them_and_moves_its_grant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        thornbite_staff(),
        jasmine_boreal(),
        16,
        &[],
        &[llanowar_elves(), llanowar_elves()],
        &[],
    );
    let granted = baylee_engine::choice::granted_ability(0);
    let offered =
        |e: &Engine<RegistryLookup>, who| priority_offer(e).abilities.contains(&(who, granted));
    assert!(!offered(&r.engine, r.host));
    r.equip_keeping(4, 12);
    assert!(offered(&r.engine, r.host));
    assert!(!offered(&r.engine, r.bystander));

    let targets = all_on_battlefield(&r.engine, p1, llanowar_elves());
    for target in targets {
        r.engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: r.host,
                    ability_index: granted,
                },
            )
            .expect("the Staff's {2}, {T} ability is offered after the previous death");
        r.engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![target],
                },
            )
            .unwrap();
        assert!(is_tapped(&r.engine, r.host), "the wearer paid {{T}}");
        pass_until(&mut r.engine, stack_is_empty);
        assert!(
            !is_tapped(&r.engine, r.host),
            "the creature's death untapped it"
        );
    }
    assert_eq!(
        r.pool(),
        8,
        "Equip {{4}} and two {{2}} activations were paid"
    );
    assert_eq!(
        r.attached_to(),
        Some(r.host),
        "pinging never sacrifices the Staff"
    );
    assert_eq!(
        r.engine.state().players[1].life,
        20,
        "both pings chose creatures rather than the player"
    );

    activate(&mut r.engine, p0, thornbite_staff(), 0);
    let Pending::ChooseTargets { options, .. } = r.engine.pending().clone() else {
        panic!("equip asks for a creature: {:?}", r.engine.pending())
    };
    assert!(options.contains(&r.host) && options.contains(&r.bystander));
    r.engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![r.bystander],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(r.attached_to(), Some(r.bystander));
    assert!(!offered(&r.engine, r.host), "the old wearer loses the ping");
    assert!(offered(&r.engine, r.bystander), "the new wearer gains it");
}

/// Any player's Shaman entry gives the Staff's controller the optional attach;
/// a non-Shaman does not. Accepting moves this same Equipment, without a target.
#[test]
fn staff_optionally_attaches_to_shamans_of_either_player_but_not_to_other_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[thornbite_staff(), forest(), forest()])
        .hand(0, &[orochi_leafcaller(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .hand(1, &[orochi_leafcaller()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let staff = on_battlefield(&e, p0, thornbite_staff()).unwrap();

    cast_from_hand(&mut e, p0, orochi_leafcaller());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!((player, prompt), (p0, YesNoPrompt::MayDo));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let shaman = on_battlefield(&e, p0, orochi_leafcaller()).unwrap();
    assert_eq!(e.state().object(staff).unwrap().attached_to, Some(shaman));

    cast_from_hand(&mut e, p0, llanowar_elves());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(staff).unwrap().attached_to, Some(shaman));

    reach_their_main_phase(&mut e, p1);
    cast_from_hand(&mut e, p1, orochi_leafcaller());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0, "the Staff's controller makes the choice");
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let theirs = on_battlefield(&e, p1, orochi_leafcaller()).unwrap();
    assert_eq!(e.state().object(staff).unwrap().attached_to, Some(theirs));
}
