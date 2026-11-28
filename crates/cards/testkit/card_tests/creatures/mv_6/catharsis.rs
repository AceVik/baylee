//! `cards/sets/2026-q1/src/ecl/catharsis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::Color;
use baylee_core::generated::subtypes;
use baylee_engine::choice::ArrangePrompt;

fn catharsis() -> CardIndex {
    card_index("9e3303aa-2484-4832-86ac-f5ae290d02f1")
}

/// Casts Catharsis for its evoke cost (`{R/W}{R/W}`) and orders its evoke
/// sacrifice trigger (ability index 0) onto the top of the stack so it resolves
/// before the enters-the-battlefield intervening-`if` trigger (`expected_etb`,
/// CR 603.3b, CR 603.4, CR 608.2h).
#[track_caller]
fn evoke_catharsis_sacrifice_first(engine: &mut Engine<RegistryLookup>, expected_etb: u32) {
    let p0 = PlayerId::new(0);
    tap_all_mana(engine, p0);
    let card = in_hand(engine, p0, catharsis()).expect("Catharsis is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("Catharsis is castable for its evoke cost");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(engine, CastModeKind::Alternative(0));
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                player,
                prompt: ArrangePrompt::TriggerOrder,
                ..
            } if *player == p0
        )
    });
    let rows = engine.queued_triggers();
    assert_eq!(
        rows.len(),
        2,
        "evoke sacrifice and the mana-color ETB trigger both queue: {rows:?}"
    );
    let sacrifice = rows
        .iter()
        .find(|t| t.loc.index == 0)
        .expect("evoke sacrifice trigger is queued");
    assert!(
        rows.iter().any(|t| t.loc.index == expected_etb),
        "expected ETB trigger {expected_etb} is queued: {rows:?}"
    );
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("waited for TriggerOrder");
    };
    let mut top_first = vec![sacrifice.handle];
    top_first.extend(cards.iter().copied().filter(|id| *id != sacrifice.handle));
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![top_first],
            },
        )
        .expect("order sacrifice above the ETB trigger");
    pass_until(engine, stack_is_empty);
}

/// Evoking Catharsis by spending `{W}{W}` and resolving the evoke sacrifice
/// trigger first still creates two 1/1 green and white Kithkin tokens: the
/// intervening-`if` check on resolution (CR 603.4) uses last known information
/// for the departed creature (CR 608.2h, CR 113.7a).
#[test]
fn catharsis_evoked_with_ww_sacrifice_first_still_creates_two_kithkin_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9021, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[catharsis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.house_rules.order_triggers = true;

    evoke_catharsis_sacrifice_first(&mut engine, 1);

    assert!(
        on_battlefield(&engine, p0, catharsis()).is_none(),
        "evoked Catharsis was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, catharsis()).is_some(),
        "Catharsis is in its owner's graveyard"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "two 1/1 green and white Kithkin tokens are created"
    );
    for token_id in tokens {
        let chars = engine
            .state()
            .object(token_id)
            .expect("token exists")
            .characteristics();
        assert_eq!(chars.power, Some(1));
        assert_eq!(chars.toughness, Some(1));
        assert!(chars.types.contains(TypeSet::CREATURE));
        assert!(chars.subtypes.contains(subtypes::creature::KITHKIN));
        assert!(chars.colors.contains(Color::White));
        assert!(chars.colors.contains(Color::Green));
    }
}

/// Evoking Catharsis by spending `{R}{R}` and resolving the evoke sacrifice
/// trigger first still gives your remaining creatures +1/+1 and haste until end
/// of turn (CR 603.4, CR 608.2h, CR 113.7a).
#[test]
fn catharsis_evoked_with_rr_sacrifice_first_still_pumps_and_gives_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9022, forest())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .hand(0, &[catharsis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.house_rules.order_triggers = true;
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears are on battlefield");

    evoke_catharsis_sacrifice_first(&mut engine, 2);

    assert!(
        on_battlefield(&engine, p0, catharsis()).is_none(),
        "evoked Catharsis was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, catharsis()).is_some(),
        "Catharsis is in its owner's graveyard"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "only red mana was spent, so no Kithkin tokens are created"
    );
    let chars = engine
        .state()
        .object(bears)
        .expect("Bears remain on the battlefield")
        .characteristics();
    assert_eq!(chars.power, Some(3), "Bears got +1/+1");
    assert_eq!(chars.toughness, Some(3), "Bears got +1/+1");
    assert!(
        chars.keywords.contains(KeywordSet::HASTE),
        "Bears gained haste until end of turn"
    );
}

/// Hard-casting Catharsis for `{4}{R/W}{R/W}` while paying `{W}{W}` keeps both
/// the 3/4 creature and the two Kithkin tokens.
#[test]
fn catharsis_hard_cast_with_ww_keeps_creature_and_creates_two_kithkin_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9023, forest())
        .battlefield(
            0,
            &[plains(), plains(), island(), island(), island(), island()],
        )
        .hand(0, &[catharsis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, catharsis()).expect("Catharsis is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("Catharsis is castable for its normal mana cost");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(&engine, CastModeKind::Normal);
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, catharsis()).is_some(),
        "hard-cast Catharsis stays on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, catharsis()).is_none(),
        "hard-cast Catharsis was not sacrificed"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "paying {{W}}{{W}} among the mana cost creates two Kithkin tokens"
    );
}
