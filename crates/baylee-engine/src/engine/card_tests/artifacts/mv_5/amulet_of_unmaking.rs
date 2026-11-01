//! `cards/artifacts/mv_5/amulet_of_unmaking.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn amulet_of_unmaking() -> CardIndex {
    card_index("945d5351-ce7d-488c-bc63-269ad82b74cb")
}

#[test]
fn amulet_of_unmaking_exiles_self_and_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                amulet_of_unmaking(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet = on_battlefield(&engine, p0, amulet_of_unmaking()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amulet_of_unmaking(), 0);
    aim_at(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(amulet).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Exile);
}

const UNLIKELY_ALLIANCE: &str = "f7ac18bc-ea63-4808-b709-92c1e69cf541";

#[test]
fn amulet_of_unmaking_exiles_only_an_artifact_creature_or_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let enchantment = card_index(UNLIKELY_ALLIANCE);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                amulet_of_unmaking(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[forest(), GRIZZLY_BEARS, sol_ring(), enchantment])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let amulet = on_battlefield(&engine, p0, amulet_of_unmaking()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let enchantment_id = on_battlefield(&engine, p1, enchantment).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amulet_of_unmaking(), 0);
    let options = aim_at(&mut engine, p0, bear);
    assert!(options.contains(&land), "a land");
    assert!(options.contains(&bear), "a creature");
    assert!(options.contains(&ring), "an artifact");
    assert!(
        !options.contains(&enchantment_id),
        "an enchantment is none of them"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(amulet).unwrap().zone, Zone::Exile);
    for untouched in [land, ring, enchantment_id] {
        assert_eq!(
            engine.state().object(untouched).unwrap().zone,
            Zone::Battlefield,
            "only the target is exiled"
        );
    }
}

#[test]
fn amulet_of_unmaking_is_sorcery_speed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                amulet_of_unmaking(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let amulet = on_battlefield(&engine, p0, amulet_of_unmaking()).unwrap();
    // p1's main phase, p0 holds priority after p1 passes: not p0's turn.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(!legal.abilities.contains(&(amulet, 0)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: amulet,
                    ability_index: 0,
                },
            )
            .is_err(),
        "activate only as a sorcery"
    );
    assert!(!is_tapped(&engine, amulet));
}
