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

/// Under a replacement that flips two coins and ignores one (CR 614.1a,
/// Krark's Thumb), the contest goes on from the result kept: no third coin
/// is flipped, the winner is the one the kept coin names, and the stake is
/// the one the contest started with. The contest used to be dropped while
/// the replacement asked, and the answer started a fresh one at a stake of
/// nought that flipped again.
#[test]
fn the_kept_result_of_a_replaced_flip_decides_the_contest() {
    use crate::engine::synthetic::{keep_mulligans, land};
    use baylee_cards_dsl::{AbilityDef, Filter, Modifier, static_ability};
    const INDEX: u32 = 19037;
    static ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::FlipTwoCoinsIgnoreOne
    )];
    let coins = |state: &GameState| -> Vec<bool> {
        state
            .journal
            .entries()
            .iter()
            .filter_map(|e| match e.event {
                GameEvent::CoinFlipped { won, .. } => Some(won),
                _ => None,
            })
            .collect()
    };
    let mut kept_each_way = [false, false];
    for seed in 60..140 {
        for ignore_first in [false, true] {
            let mut engine = crate::engine::Engine::new(
                &preset(seed, &[INDEX]),
                SyntheticLookup::new(vec![land(INDEX, "Coin doubler", ABILITIES)]),
            )
            .unwrap();
            keep_mulligans(&mut engine);
            let mut state = engine.state().clone();
            let name = state.names.intern("Coin source");
            let source = state.create_bare(
                PlayerId::new(0),
                ObjectKind::Spell,
                name,
                ZoneLocation::Stack,
            );
            let (_, mut res) = fixture(seed);
            res.source = source;
            res.on_stack = source;
            let before = coins(&state).len();
            let Flow::Wait(Pending::YesNo {
                prompt: YesNoPrompt::Generic,
                ..
            }) = run(&mut state, &mut res)
            else {
                panic!("seed {seed}: the replacement asks which coin to ignore");
            };
            let flipped = coins(&state)[before..].to_vec();
            assert_eq!(flipped.len(), 2, "seed {seed}: two coins instead of one");
            if flipped[0] == flipped[1] {
                continue;
            }
            let kept = if ignore_first { flipped[1] } else { flipped[0] };
            let Flow::Wait(Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            }) = resume_yes_no(&mut state, &mut res, ignore_first)
            else {
                panic!("seed {seed}: the winner decides next, and nothing flips again");
            };
            assert_eq!(
                coins(&state).len(),
                before + 2,
                "seed {seed}: no third coin"
            );
            let (winner, loser): (u8, u8) = if kept { (0, 1) } else { (1, 0) };
            assert_eq!(player, PlayerId::new(winner), "seed {seed}");
            assert_eq!(state.players[usize::from(winner)].life, 21, "seed {seed}");
            assert_eq!(state.players[usize::from(loser)].life, 19, "seed {seed}");
            kept_each_way[usize::from(ignore_first)] = true;
        }
        if kept_each_way == [true, true] {
            break;
        }
    }
    assert_eq!(
        kept_each_way,
        [true, true],
        "a mixed pair was decided by each of its coins"
    );
}
