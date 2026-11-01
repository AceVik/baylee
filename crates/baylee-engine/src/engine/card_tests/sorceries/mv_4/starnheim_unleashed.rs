//! `cards/sorceries/mv_4/starnheim_unleashed.rs`, ordinary and foretold.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::Color;
use baylee_core::generated::subtypes::creature;
use baylee_core::mana::ManaCost;

fn starnheim_unleashed() -> CardIndex {
    card_index("495ebe60-f0e5-4f0f-9d99-b2c64c95dbf4")
}

fn assert_angel_warriors(engine: &Engine<RegistryLookup>, player: PlayerId, count: usize) {
    let tokens = tokens_of(engine, player);
    assert_eq!(tokens.len(), count);
    for id in tokens {
        let token = engine.state().object(id).unwrap().token.unwrap();
        assert_eq!(token.name, "Angel Warrior");
        assert_eq!((token.power, token.toughness), (Some(4), Some(4)));
        assert!(token.colors.contains(Color::White));
        assert!(token.types.contains(TypeSet::CREATURE));
        assert!(token.subtypes.contains(&creature::ANGEL));
        assert!(token.subtypes.contains(&creature::WARRIOR));
        assert!(token.keywords.contains(KeywordSet::FLYING));
        assert!(token.keywords.contains(KeywordSet::VIGILANCE));
    }
}

#[test]
fn starnheim_unleashed_cast_normally_creates_one_angel_warrior() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[starnheim_unleashed()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, starnheim_unleashed());
    pass_until(&mut engine, stack_is_empty);

    assert_angel_warriors(&engine, p0, 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

fn foretold_starnheim(seed: u64) -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[starnheim_unleashed()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = in_hand(&engine, p0, starnheim_unleashed()).expect("in hand");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::Foretell { card })
        .expect("the special action pays {2}");
    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain)
            && at_rest(e, p0)
    });
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("foretold Starnheim may be cast on a later turn");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].kind, CastModeKind::Foretell);
        assert_eq!(options[0].cost, ManaCost::parse("{X}{X}{W}"));
        engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    }
    (engine, p0, card)
}

#[test]
fn starnheim_unleashed_foretold_pays_both_x_symbols_and_creates_x_tokens() {
    let (mut engine, p0, card) = foretold_starnheim(SEED);
    let Pending::ChooseNumber { player, min, .. } = engine.pending().clone() else {
        panic!("expected X, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(min, 0);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.zone, Zone::Stack);
    assert!(
        spell
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Foretold { .. }))
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "seven white mana less {{X}}{{X}}{{W}} at X = 2 leaves two"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_angel_warriors(&engine, p0, 2);
}

#[test]
fn starnheim_unleashed_foretold_with_zero_x_creates_no_tokens() {
    let (mut engine, p0, _) = foretold_starnheim(SEED + 1);
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_angel_warriors(&engine, p0, 0);
}
