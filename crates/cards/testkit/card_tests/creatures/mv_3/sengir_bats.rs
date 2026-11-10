//! Sengir Bats — `{1}{B}{B}` 1/2 Creature — Bat (Homelands)
//!
//! Flying
//! Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sengir_bats() -> CardIndex {
    card_index("ade8f21a-a7bb-4bd9-a7f2-9fa3060ff1e8")
}

fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

#[test]
fn sengir_bats_gains_plus_one_plus_one_counter_when_creature_dealt_damage_by_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[sengir_bats(), mountain()])
        .battlefield(1, &[scryb_sprites(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let bats = on_battlefield(&engine, p0, sengir_bats()).unwrap();
    let sprites = on_battlefield(&engine, p1, scryb_sprites()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    // Advance to p1's declare-attackers step; p1 attacks with 1/1 Scryb Sprites, p0 blocks with 1/2 Sengir Bats.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sprites, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bats, sprites)],
            },
        )
        .unwrap();

    // Wait until combat damage has been dealt and the trigger resolves in p1's SecondMain.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    assert_eq!(
        engine
            .state()
            .object(bats)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "Sengir Bats got a +1/+1 counter when Scryb Sprites (dealt damage by it this turn) died"
    );
    assert_eq!(pt(&engine, bats), (2, 3));

    // Killing Grizzly Bears (which was NOT dealt damage by Sengir Bats) with Lightning Bolt does not add another counter.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bats)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "Sengir Bats did not trigger when a creature not dealt damage by it died"
    );
}
