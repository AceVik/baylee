#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn virtue_of_persistence_adventure_shrinks_and_gains_life_then_enchantment_reanimates_on_upkeep() {
    let mut e = prepared(
        &[index::VIRTUE_OF_PERSISTENCE],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // Cast Locthwain Scorn (Adventure face 1).
    let card = in_hand(&e, P0, index::VIRTUE_OF_PERSISTENCE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    if let Pending::ChooseCastMode { options, .. } = e.pending().clone() {
        let idx = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Face(1)))
            .expect("Adventure face 1 offered");
        e.apply(P0, PlayerAction::ChooseMode(idx)).unwrap();
    }
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none(),
        "Locthwain Scorn gave -3/-3, killing Grizzly Bears"
    );
    assert_eq!(e.state().players[0].life, 22, "Gained 2 life");

    // Cast Virtue of Persistence from exile!
    mana(&mut e);
    let exiled = *e
        .state()
        .zones
        .list(ZoneLocation::Exile(P0))
        .first()
        .expect("on adventure in exile");
    e.apply(P0, PlayerAction::CastSpell { card: exiled })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::VIRTUE_OF_PERSISTENCE).is_some());

    // Advance to P0's next upkeep: Virtue triggers and reanimates Grizzly Bears from P1's graveyard under P0's control!
    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == P0
            && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let gy_bears = *e
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P1))
        .first()
        .unwrap();
    targets(&mut e, vec![gy_bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears reanimated under P0's control"
    );
}

#[test]
fn web_of_inertia_stops_opponent_attacks_unless_they_exile_a_graveyard_card() {
    let mut e = prepared(&[], &[index::WEB_OF_INERTIA], &[index::GRIZZLY_BEARS]);

    // P1's graveyard is empty: at beginning of combat on P1's turn, P1 cannot pay the cost, so Grizzly Bears cannot attack P0!
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::CombatBegin
            && !stack_is_empty(e)
    });
    pass_until(&mut e, stack_is_empty);

    // Since Grizzly Bears can't attack P0, the engine either offers no legal attackers or skips ChooseAttackers.
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(e.state().players[0].life, 20);
}

#[test]
fn innkeepers_talent_levels_up_grants_ward_and_doubles_counters_at_level_3() {
    let mut e = prepared(&[], &[index::INNKEEPER_S_TALENT, index::GRIZZLY_BEARS], &[]);
    let talent = on_battlefield(&e, P0, index::INNKEEPER_S_TALENT).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();

    // Level up to Level 2 ({G}, ability index 1), then Level 3 ({3}{G}, ability index 3).
    activate_by_card(&mut e, P0, index::INNKEEPER_S_TALENT, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(talent)
            .unwrap()
            .counters
            .get(CounterKind::Level),
        1
    );

    activate_by_card(&mut e, P0, index::INNKEEPER_S_TALENT, 3);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(talent)
            .unwrap()
            .counters
            .get(CounterKind::Level),
        2
    );

    // Move to beginning of combat on P0's turn: Level 1 triggers to put 1 +1/+1 counter on target creature,
    // which Level 3 doubles to 2 +1/+1 counters!
    pass_until(&mut e, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state()
            .object(bears)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "Level 3 doubled the +1/+1 counter placed on Grizzly Bears"
    );
}

#[test]
fn absolute_virtue_gives_you_protection_from_opponents_preventing_damage() {
    let mut e = prepared(&[], &[index::ABSOLUTE_VIRTUE], &[index::GRIZZLY_BEARS]);
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // Advance to P1's combat and attack P0 with Grizzly Bears unblocked.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P1, .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        e.state().players[0].life,
        20,
        "Absolute Virtue prevented combat damage from opponent's Grizzly Bears"
    );
}
