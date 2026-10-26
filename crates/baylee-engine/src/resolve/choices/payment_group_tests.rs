use super::super::{
    Amount, Effect, Flow, GameState, ObjectKind, Pending, PlayerId, PlayerRel, Resolution,
    SmallVec, ZoneLocation, resume_tax_choice, run,
};
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

static LOSS: Effect = Effect::LoseLife {
    target: PlayerRel::EventPlayer,
    amount: Amount::Fixed(1),
};

fn resolving(effects: Vec<Effect>) -> (GameState, Resolution) {
    let mut state =
        GameState::from_preset(&preset(13, &[]), &SyntheticLookup::new(vec![])).unwrap();
    state.turn.active = PlayerId::new(1);
    let name = state.names.intern("Synthetic group payment");
    let source = state.create_bare(
        PlayerId::new(0),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let res = Resolution {
        source,
        on_stack: source,
        controller: PlayerId::new(0),
        effects,
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    (state, res)
}

fn payer(flow: &Flow) -> PlayerId {
    let Flow::Wait(Pending::YesNo { player, .. }) = flow else {
        panic!("payment prompt expected")
    };
    *player
}

#[test]
fn group_payments_use_apnap_and_restore_event_after_the_last_decline() {
    let (mut state, mut res) = resolving(vec![
        Effect::PlayerMayPayOr {
            player: PlayerRel::EachPlayer,
            mana: Amount::Fixed(1),
            effect: &LOSS,
        },
        Effect::LoseLife {
            target: PlayerRel::EventPlayer,
            amount: Amount::Fixed(3),
        },
    ]);
    res.subject.set_event_player(Some(PlayerId::new(0)));
    let before = [state.players[0].life, state.players[1].life];
    assert_eq!(payer(&run(&mut state, &mut res)), PlayerId::new(1));
    assert_eq!(
        payer(&resume_tax_choice(&mut state, &mut res, false)),
        PlayerId::new(0)
    );
    assert!(matches!(
        resume_tax_choice(&mut state, &mut res, false),
        Flow::Complete
    ));
    assert_eq!(
        [state.players[0].life, state.players[1].life],
        [before[0] - 4, before[1] - 1]
    );
    assert_eq!(res.subject.event_player(), Some(PlayerId::new(0)));
    assert!(res.subject.payment_batches.is_empty());
}

#[test]
fn nested_group_payments_suspend_without_consuming_the_outer_queue() {
    static INNER: Effect = Effect::PlayerMayPayOr {
        player: PlayerRel::EachPlayer,
        mana: Amount::Fixed(2),
        effect: &LOSS,
    };
    let (mut state, mut res) = resolving(vec![Effect::PlayerMayPayOr {
        player: PlayerRel::EachPlayer,
        mana: Amount::Fixed(1),
        effect: &INNER,
    }]);
    let before = [state.players[0].life, state.players[1].life];
    let mut flow = run(&mut state, &mut res);
    for expected in [1, 1, 0, 0, 1, 0] {
        assert_eq!(payer(&flow), PlayerId::new(expected));
        flow = resume_tax_choice(&mut state, &mut res, false);
    }
    assert!(matches!(flow, Flow::Complete));
    assert_eq!(
        [state.players[0].life, state.players[1].life],
        [before[0] - 2, before[1] - 2]
    );
    assert_eq!(res.subject.event_player(), None);
    assert!(res.subject.payment_batches.is_empty());
}

#[test]
fn nested_single_player_payment_keeps_the_outer_payer_context() {
    static INNER: Effect = Effect::PlayerMayPayOr {
        player: PlayerRel::You,
        mana: Amount::Fixed(2),
        effect: &LOSS,
    };
    let (mut state, mut res) = resolving(vec![Effect::PlayerMayPayOr {
        player: PlayerRel::EachPlayer,
        mana: Amount::Fixed(1),
        effect: &INNER,
    }]);
    let before = [state.players[0].life, state.players[1].life];
    let mut flow = run(&mut state, &mut res);
    for expected in [1, 0, 0, 0] {
        assert_eq!(payer(&flow), PlayerId::new(expected));
        flow = resume_tax_choice(&mut state, &mut res, false);
    }
    assert!(matches!(flow, Flow::Complete));
    assert_eq!(
        [state.players[0].life, state.players[1].life],
        [before[0] - 1, before[1] - 1]
    );
    assert_eq!(res.subject.event_player(), None);
}

#[test]
fn a_paid_last_group_member_also_restores_the_original_event_player() {
    let (mut state, mut res) = resolving(vec![Effect::PlayerMayPayOr {
        player: PlayerRel::EachOpponent,
        mana: Amount::Fixed(0),
        effect: &LOSS,
    }]);
    res.subject.set_event_player(Some(PlayerId::new(0)));
    assert_eq!(payer(&run(&mut state, &mut res)), PlayerId::new(1));
    assert!(matches!(
        resume_tax_choice(&mut state, &mut res, true),
        Flow::Complete
    ));
    assert_eq!(res.subject.event_player(), Some(PlayerId::new(0)));
    assert!(res.subject.payment_batches.is_empty());
}
