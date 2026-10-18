//! Package 3 behavioral tests for the 10 remaining `Coverage::Generated`
//! Equipment cards under `cards/artifacts/equipment/`.

#[allow(clippy::wildcard_imports)]
use super::super::super::legends_kit::*;
use super::mv_2::{goblin_piker, rig, rig_vs};
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::types::SupertypeSet;

fn accorder_s_shield() -> CardIndex {
    // oracle_id = "dd70d439-bd60-43d1-ac26-13b744cc4a37"
    card_index("dd70d439-bd60-43d1-ac26-13b744cc4a37")
}

fn bone_saw() -> CardIndex {
    // oracle_id = "16e555f2-5aa8-4100-a036-eed48db0e84a"
    card_index("16e555f2-5aa8-4100-a036-eed48db0e84a")
}

fn cathar_s_shield() -> CardIndex {
    // oracle_id = "b270d091-ea4c-4dfd-9c50-48b45bbc396e"
    card_index("b270d091-ea4c-4dfd-9c50-48b45bbc396e")
}

fn kite_shield() -> CardIndex {
    // oracle_id = "24f9c64a-6ce6-41cd-948e-11554d7f17fb"
    card_index("24f9c64a-6ce6-41cd-948e-11554d7f17fb")
}

fn paradise_mantle() -> CardIndex {
    // oracle_id = "c1121b83-1ba2-473d-89c9-e3bbd4529072"
    card_index("c1121b83-1ba2-473d-89c9-e3bbd4529072")
}

fn spidersilk_net() -> CardIndex {
    // oracle_id = "04979f18-1ba8-47a3-a078-ab6b84bfac72"
    card_index("04979f18-1ba8-47a3-a078-ab6b84bfac72")
}

fn lion_heart() -> CardIndex {
    // oracle_id = "bd4151cc-743a-40f0-9e51-1f67555d9ab3"
    card_index("bd4151cc-743a-40f0-9e51-1f67555d9ab3")
}

fn shield_of_kaldra() -> CardIndex {
    // oracle_id = "4fdd570f-dbe0-486f-9843-f77f80956307"
    card_index("4fdd570f-dbe0-486f-9843-f77f80956307")
}

fn worldslayer() -> CardIndex {
    // oracle_id = "31be715f-9ad3-435c-9e52-e108fe96b969"
    card_index("31be715f-9ad3-435c-9e52-e108fe96b969")
}

fn meteor_sword() -> CardIndex {
    // oracle_id = "181965fb-e685-4b1f-9bd6-3a33bed933b2"
    card_index("181965fb-e685-4b1f-9bd6-3a33bed933b2")
}

#[track_caller]
fn cast_resolved(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    mana: &[(ManaColor, u32)],
) -> ObjectId {
    float(engine, seat, mana);
    let obj = in_hand(engine, seat, card).expect("card in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: obj })
        .expect("cast succeeds");
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, seat, card).expect("resolved onto battlefield")
}

#[track_caller]
fn assert_not_castable(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    let obj = in_hand(engine, seat, card).expect("card in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected Priority");
    };
    assert!(
        !legal.castable.contains(&obj),
        "card must not be castable with insufficient mana"
    );
}

#[track_caller]
fn cast_zero_cost_equipment_and_equip(
    card: CardIndex,
    equip_cost: u64,
    dp: i16,
    dt: i16,
    kw: KeywordSet,
) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(0, &[goblin_piker(), llanowar_elves()])
        .battlefield(1, &[goblin_piker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, goblin_piker()).expect("host");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("bystander");
    let theirs = on_battlefield(&engine, p1, goblin_piker()).expect("theirs");

    // Cast for {0} with empty mana pool.
    let eq = cast_resolved(&mut engine, p0, card, &[]);
    assert_eq!(
        engine.state().object(eq).and_then(|o| o.attached_to),
        None,
        "Equipment enters unattached"
    );
    assert_eq!(pt(&engine, host), (2, 1));

    // Not offered when 1 mana short of equip_cost.
    if equip_cost > 0 {
        float(
            &mut engine,
            p0,
            &[(ManaColor::Colorless, (equip_cost - 1) as u32)],
        );
        assert!(
            !priority_offer(&engine).abilities.contains(&(eq, 0)),
            "Equip {{{equip_cost}}} must not be offered 1 mana short"
        );
        float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);
    }
    assert!(
        priority_offer(&engine).abilities.contains(&(eq, 0)),
        "Equip {{{equip_cost}}} offered when paid"
    );
    activate(&mut engine, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Equip, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&host) && options.contains(&bystander));
    assert!(!options.contains(&theirs) && !options.contains(&eq));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(eq).and_then(|o| o.attached_to),
        Some(host)
    );
    assert_eq!(pt(&engine, host), (2 + dp, 1 + dt));
    assert_eq!(pt(&engine, bystander), (1, 1));
    assert_eq!(pt(&engine, theirs), (2, 1));
    if !kw.is_empty() {
        assert!(keywords(&engine, host).contains(kw));
        assert!(!keywords(&engine, bystander).contains(kw));
    }

    // Re-equip onto bystander moves the bonus and keyword.
    float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, equip_cost as u32)],
    );
    activate(&mut engine, p0, card, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(eq).and_then(|o| o.attached_to),
        Some(bystander)
    );
    assert_eq!(pt(&engine, host), (2, 1));
    assert_eq!(pt(&engine, bystander), (1 + dp, 1 + dt));
    if !kw.is_empty() {
        assert!(!keywords(&engine, host).contains(kw));
        assert!(keywords(&engine, bystander).contains(kw));
    }
}

#[test]
fn zero_cost_equipment_cast_and_equip_bonuses() {
    // Accorder's Shield — {0}: Equipped creature gets +0/+3 and has vigilance. Equip {3}.
    cast_zero_cost_equipment_and_equip(accorder_s_shield(), 3, 0, 3, KeywordSet::VIGILANCE);
    // Bone Saw — {0}: Equipped creature gets +1/+0. Equip {1}.
    cast_zero_cost_equipment_and_equip(bone_saw(), 1, 1, 0, KeywordSet::EMPTY);
    // Cathar's Shield — {0}: Equipped creature gets +0/+3 and has vigilance. Equip {3}.
    cast_zero_cost_equipment_and_equip(cathar_s_shield(), 3, 0, 3, KeywordSet::VIGILANCE);
    // Kite Shield — {0}: Equipped creature gets +0/+3. Equip {3}.
    cast_zero_cost_equipment_and_equip(kite_shield(), 3, 0, 3, KeywordSet::EMPTY);
    // Spidersilk Net — {0}: Equipped creature gets +0/+2 and has reach. Equip {2}.
    cast_zero_cost_equipment_and_equip(spidersilk_net(), 2, 0, 2, KeywordSet::REACH);
}

#[test]
#[allow(clippy::too_many_lines)]
fn paradise_mantle_grants_tap_for_any_color_to_equipped_creature() {
    // Paradise Mantle — {0}
    // Equipped creature has "{T}: Add one mana of any color."
    // Equip {1}
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[paradise_mantle()])
        .battlefield(0, &[goblin_piker(), goblin_piker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pikers = all_on_battlefield(&engine, p0, goblin_piker());
    let (host, bystander) = (pikers[0], pikers[1]);
    let mantle = cast_resolved(&mut engine, p0, paradise_mantle(), &[]);

    assert!(
        crate::effects::granted_activated(engine.state(), host)
            .next()
            .is_none(),
        "unequipped Goblin Piker has no granted mana ability"
    );

    float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);
    activate(&mut engine, p0, paradise_mantle(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(mantle).and_then(|o| o.attached_to),
        Some(host)
    );
    let host_grant = crate::effects::granted_activated(engine.state(), host)
        .next()
        .expect("equipped creature receives granted mana ability")
        .id;
    assert!(
        priority_offer(&engine)
            .abilities
            .contains(&(host, host_grant)),
        "equipped creature gains {{T}}: Add one mana of any color"
    );
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|(src, _)| *src == bystander),
        "unequipped bystander does not gain the mana ability"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: host,
                ability_index: host_grant,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert!(is_tapped(&engine, host));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );

    // Move Paradise Mantle onto bystander and tap for Black.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mantle,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        crate::effects::granted_activated(engine.state(), host)
            .next()
            .is_none(),
        "old host loses the granted ability when unequipped"
    );
    let bystander_grant = crate::effects::granted_activated(engine.state(), bystander)
        .next()
        .expect("new host receives granted mana ability")
        .id;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bystander,
                ability_index: bystander_grant,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}

#[test]
fn lion_heart_etb_deals_two_damage_and_equips_for_plus_two_plus_one() {
    // Lion Heart — {4}
    // When this Equipment enters, it deals 2 damage to any target.
    // Equipped creature gets +2/+1.
    // Equip {2}
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lion_heart()])
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[goblin_piker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    let their_piker = on_battlefield(&engine, p1, goblin_piker()).expect("their piker");

    // Negative cast at {3}.
    float(&mut engine, p0, &[(ManaColor::Colorless, 3)]);
    assert_not_castable(&engine, p0, lion_heart());
    float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);

    let card = in_hand(&engine, p0, lion_heart()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_piker],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, goblin_piker()).is_none()
            && in_graveyard(&engine, p1, goblin_piker()).is_some(),
        "Lion Heart ETB deals 2 damage to target 2/1 Goblin Piker, destroying it"
    );

    // Equip {2} onto Llanowar Elves -> 3/2.
    float(&mut engine, p0, &[(ManaColor::Colorless, 2)]);
    activate(&mut engine, p0, lion_heart(), 0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (3, 2));
}

#[test]
fn shield_of_kaldra_grants_indestructible_to_kaldra_equipment_and_equipped_creature() {
    // Shield of Kaldra — {4} — Legendary Artifact — Equipment
    // Equipment named Sword of Kaldra, Shield of Kaldra, and Helm of Kaldra have indestructible.
    // Equipped creature has indestructible.
    // Equip {4}
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[shield_of_kaldra()])
        .battlefield(0, &[bone_saw(), goblin_piker(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let saw = on_battlefield(&engine, p0, bone_saw()).expect("bone saw");
    let host = on_battlefield(&engine, p0, goblin_piker()).expect("piker");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");

    float(&mut engine, p0, &[(ManaColor::Colorless, 3)]);
    assert_not_castable(&engine, p0, shield_of_kaldra());
    float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);
    let shield = cast_resolved(&mut engine, p0, shield_of_kaldra(), &[]);

    assert!(
        engine
            .state()
            .object(shield)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );
    assert!(
        keywords(&engine, shield).contains(KeywordSet::INDESTRUCTIBLE),
        "Shield of Kaldra grants indestructible to itself"
    );
    assert!(
        !keywords(&engine, saw).contains(KeywordSet::INDESTRUCTIBLE),
        "non-Kaldra Equipment does not gain indestructible"
    );
    assert!(!keywords(&engine, host).contains(KeywordSet::INDESTRUCTIBLE));

    // Equip {4} onto Goblin Piker.
    float(&mut engine, p0, &[(ManaColor::Colorless, 4)]);
    activate(&mut engine, p0, shield_of_kaldra(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::INDESTRUCTIBLE),
        "equipped creature has indestructible"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::INDESTRUCTIBLE),
        "unequipped creature does not"
    );
}

#[test]
fn worldslayer_destroys_all_other_permanents_on_combat_damage_to_player() {
    // Worldslayer — {5}
    // Whenever equipped creature deals combat damage to a player, destroy all permanents other than this Equipment.
    // Equip {5}
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // 1. Blocked attack deals no combat damage to player -> nothing destroyed by trigger.
    let mut blocked_rig = rig_vs(
        worldslayer(),
        goblin_piker(),
        5,
        &[],
        &[llanowar_elves()],
        &[],
    );
    blocked_rig.equip(5);
    let blocker = blocked_rig.theirs;
    let attacker = blocked_rig.host;
    let Pending::ChooseBlockers { .. } = blocked_rig.attack_to_the_blocks() else {
        panic!("expected ChooseBlockers");
    };
    blocked_rig
        .engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut blocked_rig.engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        !all_on_battlefield(&blocked_rig.engine, p0, forest()).is_empty(),
        "when blocked, Worldslayer does not trigger and Forests survive"
    );

    // 2. Unblocked attack deals combat damage to player -> all other permanents destroyed, Worldslayer survives.
    let mut hit_rig = rig(worldslayer(), goblin_piker(), 5);
    let slayer = hit_rig.equipment;
    hit_rig.equip(5);
    let Pending::ChooseBlockers { .. } = hit_rig.attack_to_the_blocks() else {
        panic!("expected ChooseBlockers");
    };
    hit_rig
        .engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut hit_rig.engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    let remaining = hit_rig
        .engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .clone();
    assert_eq!(
        remaining,
        vec![slayer],
        "Worldslayer destroys all permanents other than itself on combat damage to a player"
    );
}

#[test]
fn meteor_sword_etb_destroys_target_permanent_and_equips_for_plus_three_plus_three() {
    // Meteor Sword — {7}
    // When this Equipment enters, destroy target permanent.
    // Equipped creature gets +3/+3.
    // Equip {3}
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[meteor_sword()])
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[goblin_piker(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their land");

    // Negative cast at {6}.
    float(&mut engine, p0, &[(ManaColor::Colorless, 6)]);
    assert_not_castable(&engine, p0, meteor_sword());
    float(&mut engine, p0, &[(ManaColor::Colorless, 1)]);

    let card = in_hand(&engine, p0, meteor_sword()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none()
            && in_graveyard(&engine, p1, forest()).is_some(),
        "Meteor Sword ETB destroys target permanent"
    );

    // Equip {3} onto Llanowar Elves -> 4/4.
    float(&mut engine, p0, &[(ManaColor::Colorless, 3)]);
    activate(&mut engine, p0, meteor_sword(), 0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (4, 4));
}
