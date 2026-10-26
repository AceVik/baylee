//! `cards/enchantments/auras/mv_1/sigil_of_sleep.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sigil_of_sleep() -> CardIndex {
    card_index("51b47053-d169-4627-982f-f29b8630d75f")
}

/// Sigil of Sleep (51b47053-d169-4627-982f-f29b8630d75f) — {U} Enchantment — Aura:
/// "Enchant creature"
/// "Whenever enchanted creature deals damage to a player, return target
/// creature that player controls to its owner's hand."
///
/// Triggers on any damage (including noncombat damage from Prodigal Sorcerer)
/// dealt by the enchanted creature to a player, offering only creatures that
/// damaged player controls as targets; damaging a creature instead of a player
/// does not trigger it.
#[test]
fn sigil_of_sleep_bounces_target_creature_controlled_by_the_damaged_player() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), index::PRODIGAL_SORCERER, grizzly_bears()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[sigil_of_sleep()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pinger = on_battlefield(&engine, p0, index::PRODIGAL_SORCERER).unwrap();
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, sigil_of_sleep());
    aim_at(&mut engine, p0, pinger);
    pass_until(&mut engine, stack_is_empty);

    // Negative boundary: Prodigal Sorcerer deals 1 damage to a creature (Hill Giant),
    // not a player -> Sigil of Sleep does not trigger.
    activate(&mut engine, p0, index::PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, their_giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(their_giant).unwrap().zone,
        Zone::Battlefield,
        "damaging a creature does not trigger Sigil of Sleep"
    );

    // Untap Prodigal Sorcerer and ping player P1 -> Sigil of Sleep triggers,
    // offering P1's Hill Giant and NOT P0's own creatures.
    engine.dev_state_mut(p0).unwrap().set_tapped(pinger, false);
    engine.refresh_offer();
    activate(&mut engine, p0, index::PRODIGAL_SORCERER, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&their_giant),
        "creature controlled by the damaged player is a legal target"
    );
    assert!(
        !options.contains(&my_bears) && !options.contains(&pinger),
        "creatures not controlled by the damaged player are excluded"
    );
    aim_at(&mut engine, p0, their_giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().object(their_giant).unwrap().zone,
        Zone::Hand,
        "damaged player's targeted creature is returned to its owner's hand"
    );
}
