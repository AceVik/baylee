//! `cards/artifacts/mv_6/urza_s_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_armor() -> CardIndex {
    card_index("e5fa232a-c8f2-4532-88bf-60d1223181b0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Urza's Armor (e5fa232a-c8f2-4532-88bf-60d1223181b0) — {6} Artifact:
/// "If a source would deal damage to you, prevent 1 of that damage."
///
/// Prevents 1 damage per source from both spells (Bolt 3 -> 2) and multiple
/// simultaneous attackers (two 2/2s deal 1 each, a 1/1 deals 0).
#[test]
fn urza_s_armor_prevents_one_per_source_from_spells_and_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[urza_s_armor()])
        .battlefield(
            1,
            &[
                mountain(),
                grizzly_bears(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // 1. Noncombat spell damage: Bolt deals 3 - 1 = 2 to p0 (20 -> 18).
    // Tap only the Mountain so Llanowar Elves stays untapped to attack.
    tap_all_mana_but(&mut engine, p1, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 18, "Bolt to p0 reduced from 3 to 2");

    // 2. Combat damage from three sources: 2/2 -> 1, 2/2 -> 1, 1/1 -> 0 (total 2: 18 -> 16).
    let attackers: Vec<(ObjectId, Defender)> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == p1
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .map(|id| (id, Defender::Player(p0)))
        .collect();
    assert_eq!(attackers.len(), 3);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareAttackers { attackers })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        life_of(&engine, p0),
        16,
        "each 2/2 dealt 1 and the 1/1 dealt 0 (18 -> 16)"
    );
}

/// Urza's Armor protects only YOU — neither your creatures nor your opponent.
#[test]
fn urza_s_armor_does_not_prevent_damage_to_creatures_or_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[urza_s_armor(), serra_angel(), mountain(), mountain()])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();

    // Bolt your own Serra Angel: takes full 3 damage (not prevented).
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, angel);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        3,
        "damage to your creature is not prevented"
    );

    // Bolt your opponent: takes full 3 damage (20 -> 17).
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p1),
        17,
        "damage to the opponent is not prevented"
    );
}
