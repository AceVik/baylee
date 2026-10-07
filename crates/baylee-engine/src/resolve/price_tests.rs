use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::Amount;
use baylee_core::ids::SeatSet;
use baylee_core::mana::ManaColor;

static GAIN: Effect = Effect::GainLife {
    amount: Amount::Fixed(1),
};
static PRICE: &[Effect] = &[Effect::PlayerMayPayThen {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effects: std::slice::from_ref(&GAIN),
}];
static TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effect: &GAIN,
}];
static BLUE_PRICE: &[Effect] = &[Effect::PlayerMayPayManaThen {
    player: PlayerRel::You,
    cost: baylee_core::mana!("{U}"),
    effects: std::slice::from_ref(&GAIN),
}];
static BLUE_TAX: &[Effect] = &[Effect::PlayerMayPayManaOr {
    player: PlayerRel::You,
    cost: baylee_core::mana!("{U}"),
    effect: &GAIN,
}];

fn me() -> PlayerId {
    PlayerId::new(0)
}

/// A game with one mana floating in `me`'s pool and a resolution of
/// `effects` from a bare permanent of theirs.
fn asked(effects: &'static [Effect]) -> (GameState, Resolution) {
    let (mut state, mut res) = resolving(effects, ManaColor::Colorless);
    let Flow::Wait(Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("a payment is a question put as the ability resolves (CR 608.2d)");
    };
    assert_eq!((player, mana), (me(), 1));
    (state, res)
}

/// The same, with the mana floating in `floating` and the question
/// asked for a printed price with colour in it.
fn asked_for_blue(effects: &'static [Effect], floating: ManaColor) -> (GameState, Resolution) {
    let (mut state, mut res) = resolving(effects, floating);
    let Flow::Wait(Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    }) = run(&mut state, &mut res)
    else {
        panic!("a coloured price is the same question, put with its colour");
    };
    assert_eq!((player, cost), (me(), baylee_core::mana!("{U}")));
    (state, res)
}

fn resolving(effects: &'static [Effect], floating: ManaColor) -> (GameState, Resolution) {
    let mut state = GameState::from_preset(&preset(13, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let name = state.names.intern("Crystal Rod");
    let source = state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state.players[0].mana_pool.add(floating, 1);
    let res = Resolution {
        source,
        on_stack: source,
        controller: me(),
        effects: effects.to_vec(),
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
    };
    (state, res)
}

fn life(state: &GameState) -> i32 {
    state.players[0].life
}

#[test]
fn a_price_paid_buys_the_clause_and_a_price_declined_buys_nothing() {
    let (mut state, mut res) = asked(PRICE);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, true);
    assert_eq!(
        life(&state),
        before + 1,
        "paid: \"if you do, you gain 1 life\""
    );
    assert_eq!(
        state.players[0].mana_pool.total(),
        0,
        "and the {{1}} left the pool"
    );

    let (mut state, mut res) = asked(PRICE);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, false);
    assert_eq!(life(&state), before, "declined: nothing bought");
    assert_eq!(state.players[0].mana_pool.total(), 1, "and nothing paid");
}

/// The tax is the same question with the effect on the other answer,
/// and sharing its resumption must not have turned it round.
#[test]
fn a_tax_still_runs_its_effect_on_a_refusal() {
    let (mut state, mut res) = asked(TAX);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, true);
    assert_eq!(life(&state), before, "paid: the tax's effect is avoided");

    let (mut state, mut res) = asked(TAX);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, false);
    assert_eq!(life(&state), before + 1, "refused: the effect runs");
}

/// A price with colour in it is charged as printed (CR 118.12a): the
/// blue that pays it leaves the pool, on either answer's side of the
/// pair. Which pools *can* pay is the engine's to check before it
/// answers yes (`pool_pays_tax`); `keyword_tests` plays that half.
#[test]
fn a_coloured_price_is_asked_and_paid_as_printed() {
    let (mut state, mut res) = asked_for_blue(BLUE_PRICE, ManaColor::Blue);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, true);
    assert_eq!(life(&state), before + 1, "paid: the clause is bought");
    assert_eq!(
        state.players[0].mana_pool.total(),
        0,
        "and the {{U}} is gone"
    );

    let (mut state, mut res) = asked_for_blue(BLUE_TAX, ManaColor::Blue);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, true);
    assert_eq!(life(&state), before, "paid: the tax's effect is avoided");

    let (mut state, mut res) = asked_for_blue(BLUE_TAX, ManaColor::Red);
    let before = life(&state);
    let _ = resume_tax_choice(&mut state, &mut res, false);
    assert_eq!(life(&state), before + 1, "refused: the effect runs");
    assert_eq!(
        state.players[0].mana_pool.available(ManaColor::Red),
        1,
        "and nothing was taken"
    );
}

#[test]
fn directed_spending_pays_resolution_prices_and_taxes() {
    static RED_PRICE: &[Effect] = &[Effect::PlayerMayPayManaThen {
        player: PlayerRel::You,
        cost: baylee_core::mana!("{R}"),
        effects: std::slice::from_ref(&GAIN),
    }];
    static RED_TAX: &[Effect] = &[Effect::PlayerMayPayManaOr {
        player: PlayerRel::You,
        cost: baylee_core::mana!("{R}"),
        effect: &GAIN,
    }];
    for (effects, gain) in [(RED_PRICE, 1), (RED_TAX, 0)] {
        let (mut state, mut res) = resolving(effects, ManaColor::White);
        let modifier = baylee_cards_dsl::Modifier::SpendManaAs {
            from: ManaColor::White,
            to: ManaColor::Red,
        };
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(res.source),
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier,
        });
        assert!(
            matches!(run(&mut state, &mut res), Flow::Wait(Pending::YesNo { prompt: YesNoPrompt::PayMana { cost }, .. }) if cost == baylee_core::mana!("{R}"))
        );
        assert!(crate::casting::affordable(
            &state,
            me(),
            &state.players[0].mana_pool,
            &baylee_core::mana!("{R}")
        ));
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, true);
        assert_eq!(life(&state), before + gain);
        assert!(state.players[0].mana_pool.is_empty());
    }
}
