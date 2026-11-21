//! `cards/artifacts/equipment/mv_1/blade_of_the_bloodchief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn blade_of_the_bloodchief() -> CardIndex {
    card_index("8985e545-07c3-45f9-9058-fe3c1f50de04")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn bolt_creature(engine: &mut Engine<RegistryLookup>, p0: PlayerId, target: ObjectId) {
    let bolt = in_hand(engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn equip_blade(engine: &mut Engine<RegistryLookup>, p0: PlayerId, blade: ObjectId, host: ObjectId) {
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: blade,
                ability_index: 0,
            },
        )
        .expect("equip {1}");
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

/// Blade of the Bloodchief (8985e545-07c3-45f9-9058-fe3c1f50de04) — {1} Artifact — Equipment:
/// "Whenever a creature dies, put a +1/+1 counter on equipped creature. If
/// equipped creature is a Vampire, put two +1/+1 counters on it instead."
/// "Equip {1}"
///
/// Unequipped does nothing; non-Vampire gets 1 +1/+1 counter when any creature
/// dies; Vampire gets 2 +1/+1 counters instead.
#[test]
fn blade_of_the_bloodchief_one_counter_non_vampire_two_counters_vampire() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                blade_of_the_bloodchief(),
                grizzly_bears(),
                sengir_vampire(),
                hill_giant(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lightning_bolt(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blade = on_battlefield(&engine, p0, blade_of_the_bloodchief()).unwrap();
    let p0_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let vampire = on_battlefield(&engine, p0, sengir_vampire()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // 1) Unequipped: killing Hill Giant adds 0 counters.
    tap_all_mana(&mut engine, p0);
    bolt_creature(&mut engine, p0, giant);
    assert_eq!(counters_on(&engine, p0_bears, CounterKind::P1P1), 0);
    assert_eq!(counters_on(&engine, vampire, CounterKind::P1P1), 0);

    // 2) Equipped to non-Vampire Grizzly Bears: killing opponent's Bears -> +1/+1 counter.
    equip_blade(&mut engine, p0, blade, p0_bears);
    bolt_creature(&mut engine, p0, p1_bears);
    assert_eq!(counters_on(&engine, p0_bears, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, p0_bears), (3, 3));

    // 3) Re-equipped to Sengir Vampire: killing p0's Bears -> TWO +1/+1 counters (6/6).
    equip_blade(&mut engine, p0, blade, vampire);
    bolt_creature(&mut engine, p0, p0_bears);
    assert_eq!(counters_on(&engine, vampire, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, vampire), (6, 6));
}
