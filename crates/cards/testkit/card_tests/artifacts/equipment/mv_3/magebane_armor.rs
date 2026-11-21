//! `cards/artifacts/equipment/mv_3/magebane_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn magebane_armor() -> CardIndex {
    card_index("fc24af7b-f3d0-41d0-aaac-d83424455717")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn equip_armor(engine: &mut Engine<RegistryLookup>, p0: PlayerId, host: ObjectId) {
    let armor = on_battlefield(engine, p0, magebane_armor()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: armor,
                ability_index: 0,
            },
        )
        .expect("equip {2}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Magebane Armor (fc24af7b-f3d0-41d0-aaac-d83424455717) — {3} Artifact — Equipment:
/// "Equipped creature gets +2/+4 and loses flying."
/// "Prevent all noncombat damage that would be dealt to equipped creature."
/// "Equip {2}"
///
/// Gives +2/+4, removes flying, and prevents all noncombat damage (Lightning Bolt).
#[test]
fn magebane_armor_pumps_removes_flying_and_prevents_noncombat_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                magebane_armor(),
                serra_angel(),
                plains(),
                plains(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    equip_armor(&mut engine, p0, angel);
    assert_eq!(pt(&engine, angel), (6, 8), "4/4 + 2/+4 = 6/8");
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));

    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        0,
        "Magebane Armor prevented all 3 noncombat damage from Lightning Bolt"
    );
}

/// Grounded equipped Serra Angel can be blocked by a ground creature, and
/// combat damage is NOT prevented.
#[test]
fn magebane_armor_grounded_attacker_takes_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[magebane_armor(), serra_angel(), forest(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    equip_armor(&mut engine, p0, angel);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, angel)],
            },
        )
        .expect("ground Grizzly Bears can block Serra Angel without flying");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        2,
        "combat damage is NOT prevented by Magebane Armor"
    );
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
