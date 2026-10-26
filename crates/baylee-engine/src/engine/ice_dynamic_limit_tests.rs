use super::synthetic::{SyntheticLookup, keep_mulligans, land, permanents, preset};
use super::*;
use baylee_cards_dsl::{Amount, CounterKind, Effect};

#[test]
fn source_counter_amount_modifies_only_the_printed_characteristic() {
    use super::synthetic::creature;
    use baylee_cards_dsl::{Filter, Modifier, static_ability};
    const SOURCE: u32 = 19032;
    const BODY: u32 = 19033;
    static ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::ModifyPTByAmount {
            power: Amount::CountersOnSource(CounterKind::Charge),
            toughness: Amount::Fixed(0),
        }
    )];
    let lookup = SyntheticLookup::new(vec![
        land(SOURCE, "Counter anthem", ABILITIES),
        creature(BODY, "Body", 2, 3, &[]),
    ]);
    let mut engine = Engine::new(&preset(52, &[SOURCE, BODY]), lookup).unwrap();
    keep_mulligans(&mut engine);
    let source = permanents(&engine, SOURCE)[0];
    let body = permanents(&engine, BODY)[0];
    for (counters, expected) in [(3, 5), (1, 3), (0, 2)] {
        engine
            .state
            .object_mut(source)
            .unwrap()
            .counters
            .set(CounterKind::Charge, counters);
        engine.state.invalidate_projections();
        engine.state.refresh_characteristics();
        let characteristics = engine.state.object(body).unwrap().characteristics();
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(expected), Some(3))
        );
    }
}

fn coin_contest_at_a_winners_question(won: bool) -> Engine<SyntheticLookup> {
    const INDEX: u32 = 19037;
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        Cost::TAP,
        &[Effect::FlipCoinLifeExchangeRepeat {
            opponent: baylee_cards_dsl::PlayerRel::Opponent,
            initial: Amount::Fixed(1),
            multiplier: 2,
        }]
    )];
    let mut setup = preset(57, &[INDEX]);
    setup.seats.push(setup.seats[1].clone());
    setup.seats.push(setup.seats[1].clone());
    let mut engine = Engine::new(
        &setup,
        SyntheticLookup::new(vec![land(INDEX, "Coin contest", ABILITIES)]),
    )
    .unwrap();
    // This fixture has four seats; finish every opening-hand question.
    for _ in 0..setup.seats.len() {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected an opening-hand question");
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    force_next_flip(&mut engine, won);
    let source = permanents(&engine, INDEX)[0];
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::YesNo { .. }) {
            return engine;
        }
        let pending = engine.pending().clone();
        assert!(
            super::synthetic::walk_past(&mut engine, &pending),
            "{pending:?}"
        );
    }
    panic!("coin choice was not reached");
}

fn force_next_flip(engine: &mut Engine<SyntheticLookup>, won: bool) {
    while (engine.state.rng.clone().below(2) == 0) != won {
        let _ = engine.state.rng.below(2);
    }
}

#[test]
fn a_departed_loser_can_win_the_next_flip_but_a_substitute_makes_the_repeat_choice() {
    let a = PlayerId::new(0);
    let b = PlayerId::new(1);
    let c = PlayerId::new(2);
    let d = PlayerId::new(3);
    let mut engine = coin_contest_at_a_winners_question(true);
    assert_eq!(engine.pending().asked(), Some(a));
    engine.apply(b, PlayerAction::Concede).unwrap();
    let departed_life = engine.state.players[1].life;
    force_next_flip(&mut engine, true);
    engine.apply(a, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.pending().asked(), Some(a));
    assert_eq!(engine.state.players[0].life, 23);
    assert_eq!(engine.state.players[1].life, departed_life);
    force_next_flip(&mut engine, false);
    engine.apply(a, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { player, options } if *player == a && options == &[c, d])
    );
    engine.apply(a, PlayerAction::ChoosePlayer(c)).unwrap();
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == c));
    force_next_flip(&mut engine, true);
    engine.apply(c, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.pending().asked(), Some(a));
    engine.apply(a, PlayerAction::YesNo(false)).unwrap();
    assert!(engine.resolution.is_none());
    assert_eq!(engine.state.players[0].life, 27);
    assert_eq!(engine.state.players[1].life, departed_life);
    assert_eq!(
        engine.state.players[2].life, 20,
        "the substitute only decides; they are not the life recipient"
    );
}

#[test]
fn conceding_while_holding_the_repeat_question_selects_a_substitute_without_reflipping() {
    let a = PlayerId::new(0);
    let b = PlayerId::new(1);
    let c = PlayerId::new(2);
    let d = PlayerId::new(3);
    let mut engine = coin_contest_at_a_winners_question(false);
    assert_eq!(engine.pending().asked(), Some(b));
    engine.apply(b, PlayerAction::Concede).unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { player, options } if *player == a && options == &[c, d])
    );
    engine.apply(a, PlayerAction::ChoosePlayer(c)).unwrap();
    assert_eq!(engine.pending().asked(), Some(c));
    engine.apply(c, PlayerAction::YesNo(false)).unwrap();
    assert!(engine.resolution.is_none());
    assert_eq!(
        engine
            .state
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(entry.event, GameEvent::CoinFlipped { .. }))
            .count(),
        1
    );
    assert_ne!(engine.pending().asked(), Some(b));
}

#[test]
fn a_controller_conceding_during_substitute_selection_passes_that_rule_choice_on() {
    let a = PlayerId::new(0);
    let b = PlayerId::new(1);
    let c = PlayerId::new(2);
    let d = PlayerId::new(3);
    let mut engine = coin_contest_at_a_winners_question(false);
    engine.apply(b, PlayerAction::Concede).unwrap();
    assert!(matches!(engine.pending(), Pending::ChoosePlayer { player, .. } if *player == a));
    engine.apply(a, PlayerAction::Concede).unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { player, options } if *player == c && options == &[c, d])
    );
    engine.apply(c, PlayerAction::ChoosePlayer(d)).unwrap();
    assert_eq!(engine.pending().asked(), Some(d));
    engine.apply(d, PlayerAction::YesNo(false)).unwrap();
    assert!(engine.resolution.is_none());
}
