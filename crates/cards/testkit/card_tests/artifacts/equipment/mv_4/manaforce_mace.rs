//! `cards/artifacts/equipment/mv_4/manaforce_mace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn manaforce_mace() -> CardIndex {
    card_index("7e92be29-ccd1-4556-bb2a-ac7902bc77c2")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

fn tundra() -> CardIndex {
    card_index("02418479-9455-417f-a6a1-004356faff37")
}

fn bayou() -> CardIndex {
    card_index("b76d1ae6-ad1d-4bac-b4c3-2e03e0e84d9b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn equip_mace(engine: &mut Engine<RegistryLookup>, p0: PlayerId) -> ObjectId {
    let mace = on_battlefield(engine, p0, manaforce_mace()).unwrap();
    let bears = on_battlefield(engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mace,
                ability_index: 0,
            },
        )
        .expect("equip {3}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    bears
}

/// Manaforce Mace (7e92be29-ccd1-4556-bb2a-ac7902bc77c2) — {4} Artifact — Equipment:
/// "Domain — Equipped creature gets +1/+1 for each basic land type among lands
/// you control."
/// "Equip {3}"
///
/// Zero basic types (ignoring opponent's 5), duplicate Forests = 1, plus Tundra
/// = 3, and Tundra + Bayou + Mountain = 5 (+5/+5).
#[test]
fn manaforce_mace_domain_counts_distinct_basic_land_types_you_control() {
    let p0 = PlayerId::new(0);

    // 1) Zero basic types (only Strip Mines), opponent has all 5 -> +0/+0.
    let mut e1 = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                manaforce_mace(),
                grizzly_bears(),
                strip_mine(),
                strip_mine(),
                strip_mine(),
            ],
        )
        .battlefield(1, &[plains(), island(), swamp(), mountain(), forest()])
        .start();
    keep_mulligans(&mut e1);
    reach_main_phase(&mut e1, p0);
    let b1 = equip_mace(&mut e1, p0);
    assert_eq!(e1.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        pt(&e1, b1),
        (2, 2),
        "0 basic land types among YOUR lands -> +0/+0"
    );

    // 2) Duplicate Forests -> +1/+1; playing Tundra -> +3/+3.
    let mut e2 = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                manaforce_mace(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[tundra()])
        .start();
    keep_mulligans(&mut e2);
    reach_main_phase(&mut e2, p0);
    let b2 = equip_mace(&mut e2, p0);
    assert_eq!(pt(&e2, b2), (3, 3), "3 Forests = 1 basic type -> +1/+1");
    let tun = in_hand(&e2, p0, tundra()).unwrap();
    e2.apply(p0, PlayerAction::PlayLand { card: tun }).unwrap();
    assert_eq!(
        pt(&e2, b2),
        (5, 5),
        "Forest + Tundra = 3 basic types -> +3/+3"
    );

    // 3) Full domain (Tundra + Bayou + Mountain = 5 basic types) -> +5/+5.
    let mut e3 = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                manaforce_mace(),
                grizzly_bears(),
                tundra(),
                bayou(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut e3);
    reach_main_phase(&mut e3, p0);
    let b3 = equip_mace(&mut e3, p0);
    assert_eq!(pt(&e3, b3), (7, 7), "all 5 basic land types -> +5/+5 (7/7)");
}
