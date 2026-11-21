//! `cards/artifacts/mv_2/thorn_of_amethyst.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thorn_of_amethyst() -> CardIndex {
    card_index("0c6c5336-9233-4ab1-9d55-79f20be7ea57")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Thorn of Amethyst (0c6c5336-9233-4ab1-9d55-79f20be7ea57) — {2} Artifact:
/// "Noncreature spells cost {1} more to cast."
///
/// Symmetrically taxes both players' noncreature spells by {1} (instants,
/// artifacts), while leaving creature spells (including artifact creatures)
/// untaxed.
#[test]
fn thorn_of_amethyst_taxes_noncreature_spells_for_both_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thorn_of_amethyst(), mountain(), mountain()])
        .hand(0, &[lightning_bolt(), sol_ring()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap 1 Mountain: neither {R} Bolt nor {1} Sol Ring is castable ({1}{R} and {2} needed).
    let m1 = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m1 })
        .unwrap();
    let p0_bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let ring = in_hand(&engine, p0, sol_ring()).unwrap();
    assert!(
        !priority_offer(&engine).castable.contains(&p0_bolt),
        "Lightning Bolt costs {{1}}{{R}} under Thorn of Amethyst"
    );
    assert!(
        !priority_offer(&engine).castable.contains(&ring),
        "Sol Ring costs {{2}} under Thorn of Amethyst"
    );

    // Tap the second Mountain: now {1}{R} pays for Bolt, leaving 0 mana.
    tap_all_mana(&mut engine, p0);
    assert!(priority_offer(&engine).castable.contains(&p0_bolt));
    engine
        .apply(p0, PlayerAction::CastSpell { card: p0_bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "casting Lightning Bolt spent both mana ({{1}}{{R}})"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 17);

    // Opponent with only 1 Mountain also cannot cast Lightning Bolt.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    let p1_bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    assert!(
        !priority_offer(&engine).castable.contains(&p1_bolt),
        "Thorn of Amethyst taxes opponent's noncreature spells too"
    );
}

/// Creature spells — including artifact creatures like Ornithopter ({0}) and
/// vanilla creatures like Grizzly Bears ({1}{G}) — are NOT taxed.
#[test]
fn thorn_of_amethyst_does_not_tax_creature_or_artifact_creature_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thorn_of_amethyst(), forest(), forest()])
        .hand(0, &[ornithopter(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Ornithopter ({0} Artifact Creature) is castable with 0 mana floating.
    let thopter = in_hand(&engine, p0, ornithopter()).unwrap();
    assert!(
        priority_offer(&engine).castable.contains(&thopter),
        "artifact creature spell is a creature spell, not taxed"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: thopter })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, ornithopter()).is_some());

    // Grizzly Bears ({1}{G}) casts for exactly 2 mana.
    tap_all_mana(&mut engine, p0);
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    assert!(priority_offer(&engine).castable.contains(&bears));
    engine
        .apply(p0, PlayerAction::CastSpell { card: bears })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
