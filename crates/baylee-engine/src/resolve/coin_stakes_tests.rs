use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

fn fixture(seed: u64) -> (GameState, Resolution) {
    let mut state =
        GameState::from_preset(&preset(seed, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let player = PlayerId::new(0);
    let name = state.names.intern("Escalating coin contest");
    let source = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
    let res = Resolution {
        source,
        on_stack: source,
        controller: player,
        effects: vec![Effect::FlipCoinLifeExchangeRepeat {
            opponent: PlayerRel::Chosen,
            initial: Amount::Fixed(1),
            multiplier: 2,
        }],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: Some(PlayerId::new(1)),
        target_players: SeatSet::new(),
        event_object: None,
        event_mana: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    (state, res)
}

#[test]
fn each_flip_winner_decides_and_each_continuation_doubles_the_stake() {
    let (mut state, mut res) = fixture(11);
    let mut previous = [20, 20];
    let mut flow = run(&mut state, &mut res);
    for stake in [1, 2, 4, 8] {
        let Flow::Wait(Pending::YesNo { player, .. }) = flow else {
            panic!("winner's decision");
        };
        let winner = usize::from(player.get());
        let loser = 1 - winner;
        assert_eq!(state.players[winner].life, previous[winner] + stake);
        assert_eq!(state.players[loser].life, previous[loser] - stake);
        previous = [state.players[0].life, state.players[1].life];
        flow = resume_yes_no(&mut state, &mut res, stake != 8);
    }
    assert!(matches!(flow, Flow::Complete));
    assert_eq!(
        state
            .journal
            .entries()
            .iter()
            .filter(|e| matches!(e.event, GameEvent::CoinFlipped { .. }))
            .count(),
        4
    );
}

#[test]
fn a_player_at_zero_stays_in_the_contest_until_the_instruction_finishes() {
    let (mut state, mut res) = fixture(12);
    state.players[0].life = 1;
    state.players[1].life = 1;
    let Flow::Wait(_) = run(&mut state, &mut res) else {
        panic!("first flip");
    };
    assert!(state.players.iter().any(|p| p.life == 0));
    assert!(state.players.iter().all(|p| !p.has_lost()));
    assert!(matches!(
        resume_yes_no(&mut state, &mut res, true),
        Flow::Wait(_)
    ));
    assert!(state.players.iter().all(|p| !p.has_lost()));
    assert!(matches!(
        resume_yes_no(&mut state, &mut res, false),
        Flow::Complete
    ));
}

#[test]
fn declining_a_repeat_does_not_compute_an_unrepresentable_next_stake() {
    for again in [false, true] {
        let (mut state, mut res) = fixture(15);
        res.effects[0] = Effect::FlipCoinLifeExchangeRepeat {
            opponent: PlayerRel::Chosen,
            initial: Amount::Fixed(1 << 30),
            multiplier: 2,
        };
        assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
        assert!(state.numeric_failure.is_none());
        assert!(matches!(
            resume_yes_no(&mut state, &mut res, again),
            Flow::Complete
        ));
        assert_eq!(state.numeric_failure.is_some(), again);
    }
}

#[test]
fn a_gain_replacement_finishes_before_the_opponents_loss_and_repeat_question() {
    use crate::engine::synthetic::{keep_mulligans, land};
    use baylee_cards_dsl::{AbilityDef, Filter, Modifier, ReplacementRule, static_ability};
    const INDEX: u32 = 19036;
    static ABILITIES: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::LifeGainDrawsInstead {
                who: PlayerRel::You
            }
        ),
        AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw),
    ];
    let mut engine = crate::engine::Engine::new(
        &preset(56, &[INDEX]),
        SyntheticLookup::new(vec![land(INDEX, "Replacement observer", ABILITIES)]),
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let mut state = engine.state().clone();
    state.turn.active = PlayerId::new(0);
    state.turn.step = crate::turn::Step::Draw;
    while state.rng.clone().below(2) != 0 {
        let _ = state.rng.below(2);
    }
    let name = state.names.intern("Coin source");
    let source = state.create_bare(
        PlayerId::new(0),
        ObjectKind::Spell,
        name,
        ZoneLocation::Stack,
    );
    let (_, mut res) = fixture(56);
    res.source = source;
    res.on_stack = source;
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
    assert!(matches!(res.awaiting, Some(AwaitingOp::SkipDraw { .. })));
    assert_eq!(
        state.players[1].life, 20,
        "the second life instruction has not run"
    );
    assert!(matches!(
        resume_yes_no(&mut state, &mut res, true),
        Flow::Wait(_)
    ));
    assert!(matches!(
        res.awaiting,
        Some(AwaitingOp::RepeatCoinLifeStake(_))
    ));
    assert_eq!((state.players[0].life, state.players[1].life), (20, 19));
}
