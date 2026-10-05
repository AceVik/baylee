//! Instants of the Arabian Nights / Antiquities / Legends pass (45c5bbd7),
//! each played from its Oracle text.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

/// Active Volcano: "Choose one — Destroy target blue permanent; or return
/// target Island to its owner's hand." Flash Flood is the same for red
/// permanents and Mountains.
#[test]
fn active_volcano_and_flash_flood_destroy_or_bounce() {
    // (spell, colour mana, victim colour creature, land)
    let rows = [
        (
            ids::active_volcano(),
            ManaColor::Red,
            ids::devouring_deep(),
            island(),
        ),
        (
            ids::flash_flood(),
            ManaColor::Blue,
            ids::raging_bull(),
            mountain(),
        ),
    ];
    for (n, (spell, mana, victim, land)) in rows.into_iter().enumerate() {
        // Mode 0: destroy.
        let mut e = game(
            4000 + 2 * n as u64,
            &[],
            &[victim, ids::barbary_apes(), land],
            &[spell],
            &[],
        );
        let (v, apes) = (obj(&e, P1, victim), obj(&e, P1, ids::barbary_apes()));
        float(&mut e, P0, &[(mana, 1)]);
        announce(&mut e, P0, spell, 0, 0, &[v], &[]);
        settle(&mut e);
        assert!(in_graveyard(&e, P1, victim).is_some(), "row {n}: destroyed");
        assert!(
            on_battlefield(&e, P1, ids::barbary_apes()).is_some(),
            "row {n}"
        );
        let _ = apes;

        // Mode 1: return the land.
        let mut e = game(4001 + 2 * n as u64, &[], &[victim, land], &[spell], &[]);
        let l = obj(&e, P1, land);
        float(&mut e, P0, &[(mana, 1)]);
        cast_modal(&mut e, P0, spell, 1, 0, &[l], &[]);
        assert!(in_hand(&e, P1, land).is_some(), "row {n}: bounced");
        assert!(
            on_battlefield(&e, P1, victim).is_some(),
            "row {n}: creature untouched"
        );
    }
}

/// Darkness and Holy Day: "Prevent all combat damage that would be dealt this
/// turn."
#[test]
fn darkness_and_holy_day_prevent_all_combat_damage() {
    for (n, (fog, mana)) in [
        (ids::darkness(), ManaColor::Black),
        (ids::holy_day(), ManaColor::White),
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = game(4100 + n as u64, &[ids::barbary_apes()], &[], &[fog], &[]);
        let apes = obj(&e, P0, ids::barbary_apes());
        attack(&mut e, &[apes]);
        block(&mut e, &[]);
        to_step(&mut e, P0, Step::DeclareBlockers);
        float(&mut e, P0, &[(mana, 1)]);
        cast_modal(&mut e, P0, fog, 0, 0, &[], &[]);
        through_combat(&mut e);
        assert_eq!(life(&e, P1), 20, "row {n}");
    }
}

/// Force Spike: "Counter target spell unless its controller pays {1}."
#[test]
fn force_spike_counters_unless_one_is_paid() {
    for (pay, seed) in [(false, 4200u64), (true, 4201)] {
        let mut e = game(
            seed,
            &[],
            &[],
            &[ids::barbary_apes()],
            &[ids::force_spike()],
        );
        float(
            &mut e,
            P0,
            &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        );
        announce(&mut e, P0, ids::barbary_apes(), 0, 0, &[], &[]);
        if pay {
            float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        }
        pass_once(&mut e);
        let spell = on_stack(&e, ids::barbary_apes()).expect("the Apes spell");
        float(&mut e, P1, &[(ManaColor::Blue, 1)]);
        announce(&mut e, P1, ids::force_spike(), 0, 0, &[spell], &[]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
        });
        if matches!(e.pending(), Pending::YesNo { .. }) {
            yes_no(&mut e, pay);
        }
        settle(&mut e);
        assert_eq!(
            on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
            pay,
            "pay={pay}"
        );
        assert_eq!(in_graveyard(&e, P0, ids::barbary_apes()).is_some(), !pay);
    }
}

/// Hell Swarm: "All creatures get -1/-0 until end of turn."
#[test]
fn hell_swarm_shrinks_every_creatures_power_for_the_turn() {
    let mut e = game(
        4300,
        &[ids::barbary_apes()],
        &[ids::moss_monster()],
        &[ids::hell_swarm()],
        &[],
    );
    float(&mut e, P0, &[(ManaColor::Black, 1)]);
    cast_modal(&mut e, P0, ids::hell_swarm(), 0, 0, &[], &[]);
    assert_eq!(pt(&e, obj(&e, P0, ids::barbary_apes())), (1, 2));
    assert_eq!(pt(&e, obj(&e, P1, ids::moss_monster())), (2, 6));
    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        pt(&e, obj(&e, P0, ids::barbary_apes())),
        (2, 2),
        "until end of turn only"
    );
}

/// Subdue: "Prevent all combat damage that would be dealt by target creature
/// this turn. That creature gets +0/+X until end of turn, where X is its
/// mana value."
#[test]
fn subdue_fogs_one_creature_and_toughens_it_by_its_mana_value() {
    let mut e = game(4400, &[ids::barbary_apes()], &[], &[ids::subdue()], &[]);
    let apes = obj(&e, P0, ids::barbary_apes());
    attack(&mut e, &[apes]);
    block(&mut e, &[]);
    to_step(&mut e, P0, Step::DeclareBlockers);
    float(&mut e, P0, &[(ManaColor::Green, 1)]);
    cast_modal(&mut e, P0, ids::subdue(), 0, 0, &[apes], &[]);
    assert_eq!(pt(&e, apes), (2, 4), "+0/+2: Barbary Apes has mana value 2");
    through_combat(&mut e);
    assert_eq!(life(&e, P1), 20, "its combat damage was prevented");
}

/// Alabaster Potion: "Choose one — Target player gains X life; or prevent the
/// next X damage that would be dealt to any target this turn."
#[test]
fn alabaster_potion_gains_life_or_prevents_damage_for_x() {
    let mut e = game(4500, &[], &[], &[ids::alabaster_potion()], &[]);
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
    );
    cast_modal(&mut e, P0, ids::alabaster_potion(), 0, 3, &[], &[P0]);
    assert_eq!(life(&e, P0), 23, "X = 3");

    let mut e = game(
        4501,
        &[ids::barbary_apes()],
        &[],
        &[ids::alabaster_potion(), lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
    );
    cast_modal(&mut e, P0, ids::alabaster_potion(), 1, 2, &[apes], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_modal(&mut e, P0, lightning_bolt(), 0, 0, &[apes], &[]);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "3 - 2 = 1 damage"
    );
    assert_eq!(damage_on(&e, apes), 1);
}

/// Divine Offering: "Destroy target artifact. You gain life equal to its mana
/// value."
#[test]
fn divine_offering_destroys_an_artifact_and_gains_its_mana_value() {
    let mut e = game(
        4600,
        &[],
        &[ids::black_mana_battery(), ids::barbary_apes()],
        &[ids::divine_offering()],
        &[],
    );
    let battery = obj(&e, P1, ids::black_mana_battery());
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    announce(&mut e, P0, ids::divine_offering(), 0, 0, &[battery], &[]);
    settle(&mut e);
    assert!(in_graveyard(&e, P1, ids::black_mana_battery()).is_some());
    assert_eq!(life(&e, P0), 24, "mana value 4");
}

/// Reset: "Cast this spell only during an opponent's turn after their upkeep
/// step. Untap all lands you control."
#[test]
fn reset_untaps_my_lands_but_only_after_an_opponents_upkeep() {
    let mut e = game(
        4700,
        &[island(), island(), forest()],
        &[],
        &[ids::reset()],
        &[],
    );
    let land = obj(&e, P0, forest());
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(land)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    e.refresh_offer();
    float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    assert!(!castable(&e, P0, ids::reset()), "not in my own turn");

    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    assert!(!castable(&e, P0, ids::reset()), "not during their upkeep");
    assert!(tapped(&e, land));

    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Draw
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    assert!(castable(&e, P0, ids::reset()), "after their upkeep");
    cast_modal(&mut e, P0, ids::reset(), 0, 0, &[], &[]);
    assert!(!tapped(&e, land), "every land untapped");
}

/// Shield Wall: "Creatures you control get +0/+2 until end of turn."
#[test]
fn shield_wall_toughens_my_creatures_only() {
    let mut e = game(
        4800,
        &[ids::barbary_apes()],
        &[ids::barbary_apes()],
        &[ids::shield_wall()],
        &[],
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    cast_modal(&mut e, P0, ids::shield_wall(), 0, 0, &[], &[]);
    assert_eq!(pt(&e, obj(&e, P0, ids::barbary_apes())), (2, 4));
    assert_eq!(pt(&e, obj(&e, P1, ids::barbary_apes())), (2, 2));
}

/// Transmutation: "Switch target creature's power and toughness until end of
/// turn."
#[test]
fn transmutation_switches_power_and_toughness() {
    let mut e = game(
        4900,
        &[],
        &[ids::moss_monster()],
        &[ids::transmutation()],
        &[],
    );
    let moss = obj(&e, P1, ids::moss_monster());
    float(
        &mut e,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
    );
    cast_modal(&mut e, P0, ids::transmutation(), 0, 0, &[moss], &[]);
    assert_eq!(pt(&e, moss), (6, 3));
    reach_their_main_phase(&mut e, P1);
    assert_eq!(pt(&e, moss), (3, 6), "until end of turn");
}

/// Teleport: "Cast this spell only during the declare attackers step. Target
/// creature can't be blocked this turn."
#[test]
fn teleport_makes_an_attacker_unblockable_in_the_declare_attackers_step() {
    let mut e = game(
        5000,
        &[ids::barbary_apes()],
        &[ids::moss_monster()],
        &[ids::teleport()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(&mut e, P0, &[(ManaColor::Blue, 3)]);
    assert!(!castable(&e, P0, ids::teleport()), "not in main phase");
    attackers_offered(&mut e);
    let Pending::ChooseAttackers { player, .. } = e.pending().clone() else {
        unreachable!()
    };
    e.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: vec![(apes, Defender::Player(P1))],
        },
    )
    .unwrap();
    assert_eq!(e.state().turn.step, Step::DeclareAttackers);
    float(&mut e, P0, &[(ManaColor::Blue, 3)]);
    assert!(
        castable(&e, P0, ids::teleport()),
        "in the declare attackers step"
    );
    cast_modal(&mut e, P0, ids::teleport(), 0, 0, &[apes], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        !may_block(&blockers, obj(&e, P1, ids::moss_monster()), apes),
        "it can't be blocked"
    );
}
