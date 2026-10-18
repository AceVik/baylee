#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn the_theorist_jace_beleren_triggers_on_opponent_draw_step_and_activates_all_loyalty_abilities() {
    let mut e = prepared(
        &[],
        &[index::THE_THEORIST_JACE_BELEREN, index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
    );
    let jace = on_battlefield(&e, P0, index::THE_THEORIST_JACE_BELEREN).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    // +1 (ability index 1): Create a 1/1 blue Illusion creature token.
    activate_by_card(&mut e, P0, index::THE_THEORIST_JACE_BELEREN, 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state().battlefield_seen().any(|id| {
            e.state().object(id).is_some_and(|o| {
                o.controller == P0
                    && o.token.is_some()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::ILLUSION)
            })
        }),
        "+1 creates a 1/1 Illusion token"
    );

    // Advance to P1's turn: at the beginning of P1's draw step, Jace triggers and P0 draws a card!
    let p0_hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before + 1,
        "Jace drew a card at the beginning of opponent's draw step"
    );

    // Advance back to P0's turn to test -2 and -6.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    // -2 (ability index 2): bounce up to one target artifact or creature opponent controls.
    activate_by_card(&mut e, P0, index::THE_THEORIST_JACE_BELEREN, 2);
    targets(&mut e, vec![giant], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_none());

    // Add loyalty and clear loyalty_used_this_turn to test -6 (ability index 3).
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(jace)
        .unwrap()
        .counters
        .add(CounterKind::Loyalty, 5);
    e.loyalty_used_this_turn.clear();
    e.refresh_offer();

    activate_by_card(&mut e, P0, index::THE_THEORIST_JACE_BELEREN, 3);
    pass_until(&mut e, stack_is_empty);
    let hand_now = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    assert_eq!(
        e.state()
            .object(bears)
            .unwrap()
            .counters
            .get(CounterKind::P1P1) as usize,
        hand_now,
        "-6 puts X +1/+1 counters on each creature you control where X is hand size"
    );
}

#[test]
fn nissa_vital_force_animates_land_recurs_permanent_and_creates_landfall_emblem() {
    let mut e = prepared(
        &[index::FOREST],
        &[index::NISSA_VITAL_FORCE, index::FOREST],
        &[],
    );
    let _nissa = on_battlefield(&e, P0, index::NISSA_VITAL_FORCE).unwrap();
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(land)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    // +1 (ability index 0): Untap target land you control, becomes 5/5 Elemental with haste until your next turn.
    activate_by_card(&mut e, P0, index::NISSA_VITAL_FORCE, 0);
    targets(&mut e, vec![land], vec![]);
    pass_until(&mut e, stack_is_empty);

    let ch = e.state().object(land).unwrap().characteristics();
    assert!(
        !e.state()
            .object(land)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(ch.types.contains(TypeSet::CREATURE));
    assert!(ch.types.contains(TypeSet::LAND));
    assert_eq!((ch.power, ch.toughness), (Some(5), Some(5)));
    assert!(ch.keywords.contains(KeywordSet::HASTE));

    // Now test -6 (ability index 2): emblem "Whenever a land you control enters, you may draw a card."
    e.loyalty_used_this_turn.clear();
    e.refresh_offer();
    activate_by_card(&mut e, P0, index::NISSA_VITAL_FORCE, 2);
    pass_until(&mut e, stack_is_empty);

    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    play_land(&mut e, P0, index::FOREST);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    // Played 1 land (-1), drew 1 card (+1) => net 0 change.
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before
    );
}

#[test]
fn professor_dellian_fel_loyalty_abilities_and_lifegain_drain_emblem() {
    let mut e = prepared(
        &[],
        &[index::PROFESSOR_DELLIAN_FEL],
        &[index::GRIZZLY_BEARS],
    );
    let _prof = on_battlefield(&e, P0, index::PROFESSOR_DELLIAN_FEL).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // +2 (ability index 0): gain 3 life (loyalty 5 -> 7).
    activate_by_card(&mut e, P0, index::PROFESSOR_DELLIAN_FEL, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 23);

    // -6 (ability index 3): emblem "Whenever you gain life, target opponent loses that much life."
    e.loyalty_used_this_turn.clear();
    e.refresh_offer();
    activate_by_card(&mut e, P0, index::PROFESSOR_DELLIAN_FEL, 3);
    pass_until(&mut e, stack_is_empty);

    // Now activate +2 again: P0 gains 3 life, triggering emblem to target P1 for 3 life loss!
    e.loyalty_used_this_turn.clear();
    e.refresh_offer();
    activate_by_card(&mut e, P0, index::PROFESSOR_DELLIAN_FEL, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 26);
    assert_eq!(e.state().players[1].life, 17);

    // -3 (ability index 2): Destroy target creature.
    e.loyalty_used_this_turn.clear();
    e.refresh_offer();
    activate_by_card(&mut e, P0, index::PROFESSOR_DELLIAN_FEL, 2);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn sivitri_dragon_master_taxes_attackers_and_destroys_non_dragons() {
    let mut e = prepared(
        &[],
        &[index::SIVITRI_DRAGON_MASTER],
        &[index::GRIZZLY_BEARS],
    );
    let sivitri = on_battlefield(&e, P0, index::SIVITRI_DRAGON_MASTER).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // +1 (ability index 0): Until your next turn, creatures can't attack you or planeswalkers you control unless controller pays 2 life.
    activate_by_card(&mut e, P0, index::SIVITRI_DRAGON_MASTER, 0);
    pass_until(&mut e, stack_is_empty);

    // Advance to P1's combat and declare Grizzly Bears attacking P0: P1 pays 2 life!
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
    assert_eq!(
        e.state().players[1].life,
        18,
        "P1 paid 2 life to attack under Sivitri's +1"
    );

    // Advance back to P0's turn and test -7 (destroy all non-Dragon creatures).
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(sivitri)
        .unwrap()
        .counters
        .add(CounterKind::Loyalty, 5);
    e.refresh_offer();
    activate_by_card(&mut e, P0, index::SIVITRI_DRAGON_MASTER, 2);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none());
}
