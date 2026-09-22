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
        !keywords.contains(KeywordSet::FLYING) && !keywords.contains(KeywordSet::DEATHTOUCH),
        "the strix kept {keywords:?} after its ability was countered"
    );
    assert!(
        engine.state().effects.iter().any(
            |fx| matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id, _) if id == strix)
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
            CastModeKind::Mode(m) => Some(m),
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
            .contains(KeywordSet::FLYING),
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
            CastModeKind::Mode(m) => Some(m),
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
        .list(ZoneLocation::Battlefield)
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

// ---------------------------------------------- CR 608.2b: the target re-check
//
// Found in play, which is why it is here rather than in a card file: an
// opponent cast Banishing Stroke at a creature, the creature's controller
// answered with Heroic Intervention, and the creature went to the bottom of
// the library anyway. All three cards read correctly. What was missing was
// the question CR 608.2b asks as a spell begins to resolve.
//
// The rule is quoted from the local Comprehensive Rules copy:
//
// > 608.2b If the spell or ability specifies targets, it checks whether the
// > targets are still legal. […] If all its targets, for every instance of
// > the word "target," are now illegal, the spell or ability doesn't
// > resolve. It's removed from the stack and, if it's a spell, put into its
// > owner's graveyard.
//
// Most illegal-target cases hide themselves: a target that left the
// battlefield is not findable, so the effect does nothing and only the
// journal is wrong. The visible case is the one where the object is still
// sitting there and merely no longer a legal target, which is the commonest
// protective play in Magic and the one the owner made.

/// Casts `card` from `seat`'s hand onto a stack that already holds a spell.
///
/// The passing is what makes it a *response* rather than a second spell
/// cast in an empty window, and the non-empty stack is asserted rather than
/// assumed: a test that walked past the window would cast into an empty
/// stack and prove nothing about the rule below.
#[track_caller]
fn respond_with(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    pass_until(engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
    cast_from_hand(engine, seat, card);
}

/// How many times the journal says an object left the stack each way.
///
/// Both counted together because the whole point of the pair is that they
/// are different events. `StackObjectDidNotResolve` is the one CR 603.4
/// already used and CR 608.2b now shares; `SpellCountered` is what a card
/// that cares about being countered reads, and nothing countered this.
fn how_it_left(engine: &Engine<RegistryLookup>, object: ObjectId, mark: usize) -> (usize, usize) {
    let count = |want: &GameEvent| {
        engine.journal().entries()[mark..]
            .iter()
            .filter(|e| e.event == *want)
            .count()
    };
    (
        count(&GameEvent::StackObjectDidNotResolve { object }),
        count(&GameEvent::SpellCountered { object }),
    )
}

/// A spell whose only target has gained hexproof does not resolve, and the
/// card goes to its owner's graveyard.
///
/// The journal is asserted beside the outcome and not instead of it. An
/// outcome-only test passes just as happily if the engine records
/// `SpellCountered` here — and then a card that triggers on being countered
/// would fire on a spell nobody countered, with nothing red to say so.
#[test]
fn a_spell_whose_only_target_gained_hexproof_does_not_resolve() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(73, forest())
        .battlefield(0, &[forest(), forest(), ondu_cleric()])
        .hand(0, &[heroic_intervention()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("the cleric is out");
    let mark = engine.journal().entries().len();

    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the swords' aim, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric], "the only creature on the table");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    let swords = engine.state().zones.list(ZoneLocation::Stack)[0];

    respond_with(&mut engine, p0, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, ondu_cleric()).is_some(),
        "the creature was exiled although it had hexproof when the spell \
         tried to resolve"
    );
    assert!(
        in_graveyard(&engine, p1, swords_to_plowshares()).is_some(),
        "\"…and, if it's a spell, put into its owner's graveyard\": the \
         spell is {:?}",
        engine.state().object(swords).map(|o| o.zone)
    );
    assert_eq!(
        how_it_left(&engine, swords, mark),
        (1, 0),
        "the journal has to say it did not resolve, exactly once, and has to \
         not say it was countered — nothing countered it"
    );
}

/// The other side of the same sentence: one illegal target out of two leaves
/// the spell resolving for the rest.
///
/// Curse of the Swine is the pool's one spell that genuinely stands on the
/// stack holding more than one target. It is cast for X = 2 across the
/// table — one creature on each side — so that Heroic Intervention, which
/// reaches only its caster's permanents, makes exactly one of the two
/// illegal.
///
/// What this cannot reach is the rule's own example. CR 608.2b says "for
/// every instance of the word 'target'", and a `TargetReq` in this DSL
/// carries one spec: Plague Spores' "destroy target nonblack creature and
/// destroy target land" is two instances and is not expressible here, so the
/// half of the rule that needs them is not implemented.
#[test]
fn a_spell_with_one_illegal_target_still_resolves_for_the_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elf = llanowar_elves();
    let mut engine = Duel::new(74, island())
        .battlefield(0, &[forest(), forest(), elf])
        .hand(0, &[heroic_intervention()])
        .battlefield(1, &[island(), island(), island(), island(), elf])
        .hand(1, &[curse_of_the_swine()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let mine = on_battlefield(&engine, p0, elf).expect("my elf");
    let theirs = on_battlefield(&engine, p1, elf).expect("their elf");

    cast_from_hand(&mut engine, p1, curse_of_the_swine());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![mine, theirs],
            },
        )
        .expect("two creatures is what X = 2 asks for");
    let curse = engine.state().zones.list(ZoneLocation::Stack)[0];
    assert_eq!(
        engine
            .state()
            .object(curse)
            .expect("on the stack")
            .targets
            .len(),
        2,
        "the premise: two targets, one on each side of the table"
    );

    respond_with(&mut engine, p0, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(mine).map(|o| o.zone),
        Some(Zone::Battlefield),
        "hexproof made this one an illegal target, and \"the spell won't do \
         anything to an illegal target\""
    );
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Exile),
        "the other target was still legal, so the spell resolved and exiled \
         it — an all-or-nothing check would have saved it too"
    );
    assert!(
        in_graveyard(&engine, p1, curse_of_the_swine()).is_some(),
        "a spell that resolved goes to the graveyard by the ordinary door"
    );
}

/// The negative that keeps the check honest: hexproof stops opponents only
/// (CR 702.11b), so your own spell still resolves at your own creature.
///
/// Without this, a re-check that answered "illegal" for everything would
/// pass the two tests above and break every targeted spell in the pool.
#[test]
fn your_own_spell_still_resolves_at_your_own_hexproofed_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(75, forest())
        .battlefield(0, &[forest(), forest(), plains(), plains(), ondu_cleric()])
        .hand(0, &[heroic_intervention(), swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("the cleric is out");
    let life = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, cleric).contains(KeywordSet::HEXPROOF),
        "the premise: the creature really does have hexproof now"
    );

    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the swords' aim, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![cleric],
        "hexproof keeps out opponents, so my own spell is still offered it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(cleric).map(|o| o.zone),
        Some(Zone::Exile),
        "my own Swords resolved at my own hexproofed creature"
    );
    assert_eq!(
        engine.state().players[0].life,
        life + 1,
        "\"its controller gains life equal to its power\" — the rest of the \
         spell happened too"
    );
}

// ------------------------------------------- CR 400.7: the object identity pair
//
// `object.rs`'s module header states the model this repo runs on:
//
// > Identity model: an object's `ObjectId` is stable for its whole lifetime;
// > zone changes bump `GameObject::version` (CR 400.7 — "it becomes a new
// > object"). Effects and targets that must track identity record
// > `(ObjectId, version)`; blinked permanents naturally invalidate old
// > references.
//
// `EffectFilter::ObjectIs` was the one identity-tracking site that never
// adopted it, so a created continuous effect followed its object through a
// zone change and back — which is a rules bug the pool could reach with two
// `Coverage::Implemented` cards and no synthetic anything.

/// Giant Growth on a creature that is then blinked: CR 400.7 says what comes
/// back is a new object, so the pump is not on it.
///
/// **Two creatures on one board, and that is the whole design of this test.**
/// The blinked one carries the claim; the one standing beside it carries the
/// negative, on the same turn, off the same effect table, from the same
/// spell. A version compare that fired for any reason other than a zone
/// change — and `version` is written at exactly one site in the workspace,
/// `GameState::move_object` — would be a far worse bug than the one it
/// closes, because it would quietly cancel every created effect in the pool.
/// One board answers both halves, so neither can drift from the other.
#[test]
fn a_blinked_creature_comes_back_without_the_pump_it_was_given() {
    let p0 = PlayerId::new(0);
    let cleric = ondu_cleric();
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[forest(), forest(), plains(), cleric, cleric])
        .hand(0, &[giant_growth(), giant_growth(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let clerics = all_on_battlefield(&engine, p0, cleric);
    assert_eq!(clerics.len(), 2, "two creatures: one moves, one does not");
    let (travels, stays) = (clerics[0], clerics[1]);
    let base = power_of(&engine, travels);
    assert_eq!(base, power_of(&engine, stays), "they start the same");

    for target in [travels, stays] {
        cast_from_hand(&mut engine, p0, giant_growth());
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![target],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    let pumped = power_of(&engine, travels);
    assert_eq!(
        (pumped, power_of(&engine, stays)),
        (base.map(|p| p + 3), base.map(|p| p + 3)),
        "the premise: \"+3/+3 until end of turn\" on both of them"
    );

    cast_from_hand(&mut engine, p0, ephemerate());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![travels],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        power_of(&engine, travels),
        base,
        "\"exile target creature you control, then return it\" — what came \
         back is a new object (CR 400.7) and the pump was not cast at it. \
         The id is the same one, and that is exactly why an id compare could \
         not tell"
    );
    assert_eq!(
        power_of(&engine, stays),
        pumped,
        "and the creature that did not move kept its pump: the compare has \
         to fire on a zone change and on nothing else"
    );
}

/// The negative over time rather than over the table: an effect on a
/// permanent that stays put lasts exactly as long as its duration says.
///
/// The turn is walked all the way round, so the pump is asked about after
/// every step the engine runs and not only in the window it was cast in. It
/// is the test that would catch a version compare reading the *current*
/// version on both sides, which is an id compare wearing the fix's clothes
/// and would pass the board above.
#[test]
fn a_pump_on_a_permanent_that_stays_put_lasts_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(78, forest())
        .battlefield(0, &[forest(), ondu_cleric()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("the cleric is out");
    let base = power_of(&engine, cleric).expect("a creature has power");

    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(power_of(&engine, cleric), Some(base + 3), "the premise");

    // Every step from here to the end of this turn, asked one at a time.
    let turn = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number != turn {
            break;
        }
        if engine.state().turn.step == Step::Cleanup {
            break;
        }
        assert_eq!(
            power_of(&engine, cleric),
            Some(base + 3),
            "the pump went away during {:?} of its own turn, and nothing \
             moved the creature",
            engine.state().turn.step
        );
        let pending = engine.pending().clone();
        let (player, action) = answer_one(&engine).expect("an ordinary turn asks nothing odd");
        engine
            .apply(player, action)
            .unwrap_or_else(|err| panic!("{pending:?}: {err:?}"));
    }

    pass_until(&mut engine, |e| e.state().turn.number != turn);
    assert_eq!(
        power_of(&engine, cleric),
        Some(base),
        "\"until end of turn\" still ends at the end of the turn — the \
         cleanup step is the other thing that must keep working"
    );
}

/// CR 613.1, the half a cache can lose: a finished projection is a
/// **fixpoint**, so recomputing one from scratch must return what the object
/// already carries.
///
/// `layers::recompute_with` walks one object through all the layers, so while
/// it runs, that object's *cached* characteristics are still the previous
/// projection. A modifier that counts objects and reads them off the cache
/// therefore reads its own source at the layer it had last time — and Ashaya,
/// Soul of the Wild is the printing where that is visible, because it makes
/// your nontoken creatures into lands at layer 4 and is then as big as the
/// lands you control at 7c. It has to count itself, and it came down one
/// short: cached 4/4 beside a fresh 5/5 on the very same board.
///
/// The card's own test asserts the 5/5. This one asserts the property the
/// defect broke, for every object on the board and without naming a number:
/// a second opinion may not disagree with the first. Anything counting over
/// the battlefield from inside the layer system fails here the day it reads a
/// stale self, whether or not anybody thought to write its card's test.
///
/// **And it caught a second one.** Keywords are compared beside power and
/// types because the same sentence has a second way to come out false: a
/// projection can be stale rather than mis-derived, and Steely Resolve is
/// that shape — its static was registered when the enchantment entered and
/// the creature type it reads was named one question later, which changed
/// what the filter matches without touching the effect table the generation
/// compare watches. `steely_resolve_is_read_after_the_type_is_named` below
/// is that board.
#[test]
fn a_cached_projection_is_what_a_fresh_one_would_compute() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(390, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                ashaya_soul_of_the_wild(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    // The game is walked to a quiet priority first, deliberately: the defect
    // this guards is a *stale* cache, and a board that has never been
    // refreshed twice has no stale cache to be caught with — the first
    // projection and a fresh one are then wrong in the same way and agree.
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let board = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    assert!(
        board.len() >= 7,
        "the board this is read over is {} objects, which is fewer than it \
         was built with — the reader, not the rule",
        board.len()
    );
    for id in board {
        let object = engine.state().object(id).expect("the object is seated");
        let fresh = crate::layers::recompute(engine.state(), object);
        assert_eq!(
            (fresh.characteristics.power, fresh.characteristics.toughness),
            (
                object.characteristics().power,
                object.characteristics().toughness
            ),
            "{:?} projects differently the second time, so the cache is not a \
             fixpoint",
            object.card.map(|c| c.index)
        );
        assert_eq!(
            fresh.characteristics.types,
            object.characteristics().types,
            "{:?} changes type on a recompute",
            object.card.map(|c| c.index)
        );
        assert_eq!(
            fresh.characteristics.keywords,
            object.characteristics().keywords,
            "{:?} gains or loses a keyword on a recompute",
            object.card.map(|c| c.index)
        );
    }
}

/// The same property on the board that showed its second face: a choice only
/// a **filter** reads still has to refresh the board.
///
/// Steely Resolve's "creatures of the chosen type have shroud" is one static,
/// registered as the enchantment enters — before anybody has been asked which
/// type. Naming the type changes which permanents the filter matches and
/// nothing about the effect table, and the refresh is guarded by a generation
/// compare over that table, so every creature kept the projection it had from
/// before the question: the Elf beside it had no shroud, and a fresh
/// recompute of the very same object said it did.
///
/// This is the played half. `Engine::apply`'s `ChooseSubtype` arm calls
/// `invalidate_projections`, and without that call this test reads the
/// difference as a disagreement between the cache and a recompute.
#[test]
fn steely_resolve_is_read_after_the_type_is_named() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(405, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[steely_resolve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    cast_from_hand(&mut engine, p0, steely_resolve());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .expect("Elf is a creature type");
    pass_until(&mut engine, stack_is_empty);

    let object = engine.state().object(elf).expect("the Elf is still there");
    let fresh = crate::layers::recompute(engine.state(), object);
    assert_eq!(
        object.characteristics().keywords,
        fresh.characteristics.keywords,
        "the cache and a recompute disagree, so naming the type refreshed \
         nothing"
    );
    assert!(
        object
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::SHROUD),
        "and both of them say shroud, which is what the card prints"
    );
}

/// A cost that asks the same question five times needs five answers on the
/// table before the ability is offered at all.
///
/// `can_afford` used to ask each `CostPart::Sacrifice` whether *anything*
/// could pay it, which is true of all five parts while one artifact stands
/// there — and `cost_wizard` takes each answer out of the list before asking
/// again, so the second question found nothing and the activation was refused
/// after being offered. `offer_tests::every_offered_ability_can_be_activated`
/// found it on the pool sweep's own board; this is the played half, with the
/// number moved by one across the line.
///
/// Time Sieve is the example and not the subject: it is the pool's only
/// printing that asks one question five times.
#[test]
fn an_ability_asking_one_question_five_times_needs_five_answers() {
    let p0 = PlayerId::new(0);
    let offered = |artifacts: usize| {
        let mut board = vec![time_sieve()];
        board.extend(std::iter::repeat_n(quiet_artifact(), artifacts));
        let mut engine = Duel::new(404, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let sieve = on_battlefield(&engine, p0, time_sieve()).expect("the Sieve is seated");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal.abilities.iter().any(|(id, _)| *id == sieve)
    };

    // Four artifacts beside the Sieve is five artifacts in total — one of
    // which is the Sieve, and CR 701.16a lets it sacrifice itself — so the
    // line sits between three and four.
    assert!(
        !offered(3),
        "four artifacts cannot pay for five sacrifices, so the ability is not \
         offered"
    );
    assert!(
        offered(4),
        "and five can, so the board and not the reader is what moved"
    );
}

fn giant_tortoise() -> CardIndex {
    card_index("2dd50d7f-941f-4deb-a15c-ee2357844c35")
}

/// The same property again, one input further out: a **tap** changes which
/// permanents a filter matches, and nothing about the effect table.
///
/// Giant Tortoise prints "this creature gets +0/+3 as long as it's untapped",
/// one `static_ability!` over `Filter::And(&[This, Untapped])`. Attacking
/// taps it (CR 508.1f) and the projection is guarded by a generation compare
/// over the *effects*, so the 1/4 stayed a 1/4 while it was lying down:
/// `GameState::set_tapped` is the door that says otherwise, and without it
/// the cache and a fresh recompute of the very same object disagree.
///
/// Read as a rule rather than as this card. Nine other cards in the pool read
/// their own tap status through a static — Castle, Spectral Cloak and the six
/// storage lands — and every one of them was answering out of a stale cache.
/// Seventeen sites wrote that bit before there was one door.
#[test]
fn a_static_that_reads_the_tap_is_refreshed_when_the_permanent_taps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(407, forest())
        .battlefield(0, &[giant_tortoise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let turtle = on_battlefield(&engine, p0, giant_tortoise()).expect("the Turtle is seated");
    assert_eq!(
        pt(&engine, turtle),
        (1, 4),
        "a printed 1/1 standing up: +0/+3 while untapped"
    );

    // Its controller's next turn, so that summoning sickness is no longer
    // what is being measured. Attacking is the tap: no vigilance is printed
    // on this card, so declaring it is CR 508.1f taking it down.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { attackers, .. } if attackers.contains(&turtle)
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(turtle, Defender::Player(p1))],
            },
        )
        .expect("a Turtle that has been out since the turn began may attack");
    assert!(is_tapped(&engine, turtle), "attacking tapped it");

    let object = engine.state().object(turtle).expect("still on the table");
    let fresh = crate::layers::recompute(engine.state(), object);
    assert_eq!(
        (
            object.characteristics().power,
            object.characteristics().toughness
        ),
        (fresh.characteristics.power, fresh.characteristics.toughness),
        "the cache and a recompute disagree, so the tap refreshed nothing"
    );
    assert_eq!(
        pt(&engine, turtle),
        (1, 1),
        "and both of them say the printed 1/1: the +0/+3 is gone with the \
         untapped it was conditioned on"
    );
}

/// `GameState::set_tapped` is the only writer of `Status::TAPPED`.
///
/// The rule above is a property of one door, and a door is only a door while
/// nothing walks past it — so this counts the writers rather than asserting
/// there are none elsewhere. A zero would pass over a tree this scan could
/// not read at all, which is how eleven textual readers of the card pool were
/// found answering a question they could not see.
///
/// It names **files** and a count, never line numbers: a reference that
/// drifts on every unrelated edit to `state.rs` would go red by itself and be
/// raised rather than read.
#[test]
fn nothing_writes_the_tapped_bit_except_the_one_door() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut writers: Vec<String> = Vec::new();
    let mut files = 0usize;
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the crate's own source is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&path).expect("a source file is readable");
            for line in text.lines() {
                // The spelling the `use`d and the fully-qualified forms share.
                let writes = line.contains("status.insert(") || line.contains("status.remove(");
                if writes && line.contains("TAPPED") {
                    writers.push(
                        path.strip_prefix(&root)
                            .expect("under src")
                            .display()
                            .to_string(),
                    );
                }
            }
        }
    }
    // A recursive walk and not a glob: `src/engine/` is two levels down and a
    // non-recursive read would have found the bit nowhere and said so.
    assert!(
        files > 40,
        "the walk read {files} files, which is not this crate"
    );
    writers.sort();
    assert_eq!(
        writers,
        // Twice `state.rs` for the two lines of `GameState::set_tapped`, and
        // once `object.rs` for its own test that the bit-flag set holds a bit.
        ["object.rs", "state.rs", "state.rs"],
        "those three are the whole list. A new entry is a site that taps a \
         permanent without telling the layer projection, which is what \
         `set_tapped` exists to stop — route it through the door instead of \
         widening this list."
    );
}

fn orcish_oriflamme() -> CardIndex {
    card_index("0b16a650-68b0-44dc-a9e1-15b7966e0b18")
}

/// The other half of the same rule, on the other board-state filter.
///
/// Orcish Oriflamme is "attacking creatures you control get +1/+0" — one
/// `static_ability!` over `Filter::Attacking`. Declaring an attacker changes
/// which permanents that filter matches and touches no effect, so the anthem
/// applied to nothing at all: the Elf swung as a 1/1 while a fresh recompute
/// of the very same object said 2/1.
///
/// It is written beside the tap because they are one finding. Tap status and
/// attack status are the two inputs the layer system reads off the *board*
/// rather than off an effect, `state::filter_reads_board_state` names them
/// together, and `GameState::board_state_changed` is the single gate both go
/// through — so a fix for one that left the other alone would have been a
/// sentence true at one and broken at two.
#[test]
fn an_anthem_that_reads_attacking_is_refreshed_when_the_attack_is_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(408, forest())
        .battlefield(0, &[llanowar_elves(), orcish_oriflamme()])
        .start();
    keep_mulligans(&mut engine);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 standing at home: the anthem names attackers"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { attackers, .. } if attackers.contains(&elf)
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .expect("an Elf that has been out since the turn began may attack");

    let object = engine.state().object(elf).expect("still on the table");
    let fresh = crate::layers::recompute(engine.state(), object);
    assert_eq!(
        object.characteristics().power,
        fresh.characteristics.power,
        "the cache and a recompute disagree, so declaring the attack \
         refreshed nothing"
    );
    assert_eq!(
        pt(&engine, elf),
        (2, 1),
        "and both of them say 2/1, which is what the enchantment prints"
    );
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// "Any target" is reachable while a player is sitting there, and the
/// castable offer has to say so.
///
/// `Pending::ChooseTargets` carries two lists — `options` and
/// `player_options` — and CR 115.4 picks one target out of their union. The
/// gate that decides whether a spell is offered at all counted only the
/// first, and `TargetSpec::AnyTarget` was short-circuited to "reachable" by
/// name beside `AnyPlayer` and `AnyOpponent`, which is what hid it. With the
/// name removed the count was left alone with the objects: Lightning Bolt
/// became uncastable at a board holding no creature, with the opponent on the
/// menu the wizard would have printed the moment it was pressed. Sixty-one
/// cards in this pool say "any target".
///
/// The empty board is the whole scenario, because a board with any creature
/// on it answers the question through the wrong list.
#[test]
fn a_spell_that_may_target_a_player_is_offered_on_a_creatureless_board() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .all(|id| engine
                .state()
                .object(*id)
                .is_some_and(|o| !o.characteristics().types.contains(TypeSet::CREATURE))),
        "the premise: not one creature anywhere, so the object list is empty"
    );

    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, lightning_bolt()).expect("the Bolt is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "an opponent is a legal target for \"any target\", so the spell is \
         offered: {:?}",
        legal.castable
    );

    // And pressing it agrees, which is the half that says the offer was not
    // merely generous: an offer the wizard then refuses is the same defect
    // wearing the other face.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the Bolt targets, got {:?}", engine.pending())
    };
    assert!(options.is_empty(), "no object is on the menu: {options:?}");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "both seats are (CR 115.4): {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "three damage, at a board where the spell could not be cast at all"
    );
}
