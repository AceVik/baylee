//! Scenarios that reach a **rule** rather than a card. The test still
//! plays a printing, because the engine advances no other way, but what it
//! asserts is the engine's behaviour and the card is whatever was nearest to
//! hand -- so a card named here is an example and not the subject.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Tishana's Tidebinder: "counter up to one target activated or triggered
/// ability. If an ability of an artifact, creature, or planeswalker is
/// countered this way, that permanent loses all abilities for as long as
/// this creature remains on the battlefield."
///
/// The second sentence had never once fired. Both halves read the same
/// target, and the first half removes the countered ability from the arena
/// before the second half looks it up — so it found nothing, registered
/// nothing, and Baleful Strix kept its flying and its deathtouch with a
/// Tidebinder standing over it.
///
/// The keywords are what this asserts because they are what the engine can
/// take away; the rest of the sentence is the `NOT SUPPORTED` note on the
/// card. Both halves are checked: the effect has to be registered *against
/// the Strix*, because a rider aimed at nothing leaves exactly the same
/// keywords standing on a creature that happens to have none.
#[test]
fn tishanas_tidebinder_strips_the_permanent_whose_ability_it_countered() {
    let (engine, _p0, _p1, strix) = a_strix_the_tidebinder_answered();
    let keywords = keywords_of(&engine, strix);
    assert!(
        !keywords.contains(baylee_cards_dsl::KeywordSet::FLYING)
            && !keywords.contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH),
        "the strix kept {keywords:?} after its ability was countered"
    );
    assert!(
        engine.state().effects.iter().any(
            |fx| matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id) if id == strix)
        ),
        "nothing was registered against the strix, so the keywords went \
         somewhere else or were never there"
    );
}

/// The question: all three of Aether Channeler's modes, offered to its
/// controller as the trigger goes on the stack (CR 603.3c).
#[test]
fn a_modal_trigger_offers_every_mode_it_can_legally_choose() {
    let p0 = PlayerId::new(0);
    let engine = a_modal_trigger_asks(53, &[quiet_creature()]);
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the helper returns standing on the question")
    };
    assert_eq!(player, p0, "the trigger's controller chooses the mode");
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1, 2],
        "a Bird, a bounce and a draw — the bounce is legal because the \
         opponent has a nonland permanent to point it at",
    );
}

/// The first answer: the mode with no targets resolves on its own.
#[test]
fn a_modal_trigger_resolves_the_mode_that_was_chosen() {
    let p0 = PlayerId::new(0);
    let mut engine = a_modal_trigger_asks(59, &[quiet_creature()]);
    let before = tokens_of(&engine, p0).len();
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), before + 1, "mode 0 makes one Bird token");
    assert!(
        engine
            .state()
            .object(*tokens.last().expect("the Bird"))
            .expect("the token exists")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "a 1/1 white Bird with flying",
    );
}

/// The second answer, and the half that `actions.rs` used to skip: a mode
/// that targets asks for *its own* target, not the ability's.
#[test]
fn a_modal_trigger_asks_for_the_targets_of_its_chosen_mode() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_modal_trigger_asks(61, &[quiet_creature()]);
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the opponent's creature");
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the bounce mode asked for no target — got {:?}. The mode carries \
             the `TargetReq`, and reading it off the ability finds none.",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        options.contains(&elves),
        "the opponent's creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p1, quiet_creature()).is_some(),
        "\"return another target nonland permanent to its owner's hand\"",
    );
}

/// The mode that cannot be chosen: with nothing else on the battlefield the
/// bounce has no legal target, so CR 603.3c takes it off the list rather
/// than offering a choice that resolves to nothing.
#[test]
fn a_mode_with_no_legal_target_is_not_offered() {
    let engine = a_modal_trigger_asks(67, &[]);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("the helper returns standing on the question")
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 2],
        "the Bird and the draw; \"another target nonland permanent\" finds \
         nothing on a board of three Islands and the Channeler itself",
    );
}

/// Hagra Diabolist: "you **may** have target player lose life equal to the
/// number of Allies you control."
///
/// Three cards in the pool said `PlayerRel::Opponent` where their printing
/// says "target player", and this is one of them. That relation is
/// `EachOpponent` in `eval::players`, so the ability drained every opponent
/// at once and could never be pointed at the controller — both invisible in
/// a duel, where "each of them" and "the one you chose" are the same seat.
///
/// The "may" is the target count, the way Sun Titan's "you may return
/// target …" is written here: `min` of nought is the decline.
///
/// Two tests and not two arms of one, because the second cast would want
/// five untapped Swamps a second time and `walk_to_own_main` returns at once
/// when it is already there — a whole turn of passing to prove a second
/// thing the first game has nothing to do with.
#[test]
fn hagra_diabolist_drains_the_player_it_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = hagra_on_the_table();
    let (life0, life1) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the builder stops at the target choice")
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "\"target player\" is every seat, the controller included",
    );
    assert_eq!((min, max), (0, 1), "\"you may\" is the nought in the min");

    // Pointed at the controller's own seat, which the card could not do at
    // all before: `PlayerRel::Opponent` is every opponent and never you.
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

    let allies = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == p0
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::ALLY)
            })
        })
        .count();
    assert!(allies > 0, "the Diabolist counts itself");
    assert_eq!(
        engine.state().players[0].life,
        life0 - allies as i32,
        "the seat it named lost one life per Ally",
    );
    assert_eq!(
        engine.state().players[1].life,
        life1,
        "and the seat it did not name lost nothing",
    );
}

/// The other half of the "may": declining the target declines the effect.
#[test]
fn hagra_diabolist_may_be_declined() {
    let mut engine = hagra_on_the_table();
    let p0 = PlayerId::new(0);
    let (life0, life1) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (life0, life1),
        "nobody was named, so nobody lost anything",
    );
}

/// "You may gain life equal to the number of Allies you control" — taken,
/// that is one Ally and one life.
#[test]
fn an_optional_rally_trigger_pays_when_it_is_taken() {
    let p0 = PlayerId::new(0);
    let mut engine = a_cleric_asking();
    let before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before + 1,
        "one Ally on the battlefield is one life"
    );
}

/// The other half of the printed "may", and the half the card was written
/// without: declining costs the player nothing and gains them nothing.
#[test]
fn an_optional_rally_trigger_may_be_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = a_cleric_asking();
    let before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before,
        "a declined 'may' does nothing at all"
    );
}
