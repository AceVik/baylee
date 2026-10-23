//! `cards/instants/mv_2/suffocation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn suffocation() -> CardIndex {
    card_index("57aeb106-dc4f-4069-b596-f997263dc0cf")
}

/// Whether the player holding priority may cast `spell` right now: in
/// `legal.castable` when the mana already floats, in `legal.payable` when
/// every other rule holds and the lands still have to be tapped. Only the
/// union answers "may cast", and a negative asserted on `castable` alone
/// would pass for the wrong reason (no mana floating).
fn castable(engine: &Engine<RegistryLookup>, spell: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority, got {:?}", engine.pending())
    };
    legal.castable.contains(&spell) || legal.payable.contains(&spell)
}

/// Suffocation ({1}{U} Instant):
/// - "Cast this spell only if you were dealt damage this turn by a red instant or sorcery spell."
#[test]
fn suffocation_is_not_castable_without_red_instant_or_sorcery_damage_from_an_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain()])
        .hand(0, &[suffocation(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, suffocation()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    // Mana is not what is missing: the Bolt beside it can be cast.
    assert!(castable(&engine, bolt));
    assert!(!castable(&engine, spell), "no damage was dealt to you");

    // Damage from a red instant, but one the caster controls: not "dealt
    // damage by" the kind of spell that this card counts.
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);
    assert!(!castable(&engine, spell), "your own Bolt does not count");
    assert_eq!(engine.state().players[1].life, 20);
}

/// Suffocation — "Suffocation deals 4 damage to the controller of the last red
/// instant or sorcery spell that dealt damage to you this turn. Draw a card at
/// the beginning of the next turn's upkeep."
#[test]
fn suffocation_deals_four_damage_to_the_opponent_who_burned_you_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[mountain()])
        .hand(0, &[suffocation()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    // The opponent's Lightning Bolt (a red instant) deals 3 to P0.
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 17);
    let spell = in_hand(&engine, p0, suffocation()).unwrap();
    assert!(
        castable(&engine, spell),
        "damage by a red instant was dealt"
    );

    cast_from_hand(&mut engine, p0, suffocation());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        16,
        "4 damage to the opponent"
    );
    assert_eq!(engine.state().players[0].life, 17);

    // The card arrives at the next turn's upkeep, besides the turn's draw.
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_cast + 2
    );
}
