//! `cards/creatures/mv_4/clone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Academy Rector ({3}{W}, 1/2): "When this creature dies, you may exile it.
/// If you do, search your library for an enchantment card, put that card
/// onto the battlefield, then shuffle."
///
/// One printed "may" gates both halves, and the gate is the thing worth
/// playing: the exile is what the tutor is paid with, so a Rector that stays
/// in its graveyard must also leave the library alone. Both answers are
/// struck on boards identical up to the question.
///
/// The enchantment has to arrive on the **battlefield** and not in hand,
/// which is why the last assertion reads the found card's own zone: p0's
/// hand is full of Luminarch Ascensions off the filler deck, so "is one in
/// hand" was already true before the Rector ever died and would have passed
/// against a card that fetched to the wrong place.
#[test]
fn the_rectors_enchantment_arrives_only_when_it_exiles_itself() {
    // Declined: the Rector lies where it fell, and nothing is searched for.
    let (mut engine, p0) = a_rector_asking();
    let library_before = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, academy_rector()).is_some(),
        "a declined \"you may exile it\" leaves the Rector in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, luminarch_ascension()).is_none(),
        "and \"if you do\" fetches no enchantment at all"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the library was never searched"
    );

    // Taken: the Rector exiles itself, and that buys the enchantment.
    let (mut engine, p0) = a_rector_asking();
    let library_before = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop above waited for exactly this")
    };
    assert_eq!(
        (min, max),
        (1, 1),
        "\"search your library for an enchantment card\" finds one card and \
         is not an \"up to\""
    );
    let found = *options
        .first()
        .expect("the library is nothing but enchantments");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search itself offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, academy_rector()).is_none(),
        "the Rector paid the exile it offered"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Exile(p0))
            .iter()
            .copied()
            .any(|id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == academy_rector()))),
        "and it is exiled rather than merely gone"
    );
    assert_eq!(
        engine
            .state()
            .object(found)
            .expect("the card the search found")
            .zone,
        crate::zone::Zone::Battlefield,
        "the enchantment is put onto the battlefield, not into hand"
    );
    assert!(
        !engine
            .state()
            .object(found)
            .expect("the card the search found")
            .status
            .contains(crate::object::Status::TAPPED),
        "and it is put onto the battlefield, not onto the battlefield tapped \
         — the printed sentence names no tapping, and `Find::BATTLEFIELD` is \
         the half of the pair that says so"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it came out of the library"
    );
}

/// Deathrite Shaman ({B/G}, 1/2): "{T}: Exile target land card from **a**
/// graveyard. Add one mana of any color. (Activate only as an instant.)"
///
/// Three claims live in that one printed line and none of them had ever been
/// played. The first is the reminder text: an ability that targets is not a
/// mana ability however much mana it makes (CR 605.1a), so the Shaman has to
/// be absent from `mana_abilities` and present in `abilities`, and the mana
/// only arrives once the thing has gone on the stack and resolved. The
/// Forest standing beside it is what makes that reading non-vacuous — the
/// list is not empty, the Shaman is simply not in it.
///
/// The second is "**a** graveyard", which the card spells
/// `PlayerRel::EachPlayer`: both graveyards are seeded and both land cards
/// have to be on the offer, or the Shaman is a card that can only eat its
/// own yard. The one it is pointed at is the opponent's, and mine is checked
/// afterwards to catch the opposite fault — a relation resolved as "every
/// player" would empty both and pass a test that only looked at theirs.
///
/// The third is "one mana of any color". Red is neither of the Shaman's own
/// colours, so a pool holding {R} says the colour came from the choice and
/// not from its identity — and the Forest is still untapped, so it did not
/// come from the land either.
#[test]
#[allow(clippy::too_many_lines)] // both graveyards have to be seeded and both halves of the ability played in one duel
fn deathrite_shaman_eats_a_land_out_of_either_graveyard_and_pays_a_colour_it_is_not() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[deathrite_shaman(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A land card in each graveyard: the filler deck is Forests, so this is
    // what gives the ability something legal to point at on both sides.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let my_land = in_graveyard(&engine, p0, forest()).expect("a land card of my own is buried");
    let their_land = in_graveyard(&engine, p1, forest()).expect("and one of theirs");

    let shaman = on_battlefield(&engine, p0, deathrite_shaman()).expect("the Shaman is out");
    let land = on_battlefield(&engine, p0, forest()).expect("and a Forest beside it");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "the Forest is what a mana ability looks like in this list, so the \
         Shaman's absence from it below is a reading and not an empty list"
    );
    assert!(
        !legal.mana_abilities.contains(&shaman),
        "an ability that targets is no mana ability (CR 605.1a), whatever it \
         adds: {:?}",
        legal.mana_abilities
    );
    assert!(
        legal.abilities.contains(&(shaman, 0)),
        "it is offered as an ordinary activated ability instead: {:?}",
        legal.abilities
    );

    // Nothing is tapped for mana first: {T} is the entire cost.
    activate(&mut engine, p0, deathrite_shaman(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which land card, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&my_land),
        "\"a graveyard\" includes my own: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "and it reaches across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: vec![],
            },
        )
        .expect("a land card in the opponent's graveyard is a legal target");

    // The mana arrives on resolution, not on activation — so the colour is
    // asked after both seats have passed on an ability sitting on the stack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    assert_eq!(player, p0, "the Shaman's controller picks the colour");
    assert!(
        options.contains(&ManaColor::Red),
        "\"any color\" is not the card's own two: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red is one of the colours that were offered");

    assert!(
        in_graveyard(&engine, p1, forest()).is_none(),
        "the land card it named left that graveyard"
    );
    let exiled: Vec<baylee_core::ids::ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p1))
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(
        exiled.len(),
        1,
        "and it is exiled under its owner, not merely gone"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "one card was targeted and only that one moved"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Exile(p0))
            .is_empty(),
        "my own graveyard was on the offer and was not the answer"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "and it is the colour that was named"
    );
    assert!(
        engine
            .state()
            .object(shaman)
            .expect("the Shaman is still there")
            .status
            .contains(crate::object::Status::TAPPED),
        "paying {{T}} left it tapped"
    );
    assert!(
        !engine
            .state()
            .object(land)
            .expect("the Forest is still there")
            .status
            .contains(crate::object::Status::TAPPED),
        "the Forest never paid for any of this, so the mana in the pool is \
         the Shaman's"
    );
}

/// The half the card cannot do, struck where it would show.
///
/// "{1}{G}{W}{U}: Put Derevi onto the battlefield from the command zone" is
/// the `Coverage::Partial` gap, and the card file is explicit that it is not
/// written at all rather than written and skipped: `ActivationZone` names the
/// battlefield and your hand and nothing else. So she is seated as a
/// commander with four lands tapped for {G}{W}{U}{G} — which pays
/// {1}{G}{W}{U} exactly, so the ability is missing from the offer because
/// nobody wrote it and not because nobody could afford it — and the engine
/// offers precisely one thing to do with the card in that zone: cast her
/// (CR 903.8). Both halves are needed: the cast is what proves `LegalActions`
/// can see a command-zone object at all, without which "no ability is
/// offered" would be true of a card the engine had never looked at.
#[test]
fn derevi_leaves_the_command_zone_by_being_cast_and_by_no_ability_of_her_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .commander(0, &[derevi_empyrial_tactician()])
        .battlefield(0, &[forest(), plains(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_island = on_battlefield(&engine, p0, island()).expect("an Island of her own");
    let command_zone = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Command(p0))
        .clone();
    assert_eq!(command_zone.len(), 1, "Derevi starts in the command zone");
    let derevi = command_zone[0];

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&derevi),
        "the engine sees the card in the command zone and offers the cast",
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == derevi),
        "and offers no ability on it: \"{{1}}{{G}}{{W}}{{U}}: Put Derevi onto \
         the battlefield from the command zone\" is the gap this card's \
         `Coverage::Partial` names, so she plays exactly as though that line \
         were not printed",
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: derevi })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && on_battlefield(e, p0, derevi_empyrial_tactician()).is_some())
    });
    tap_or_untap(&mut engine, p0, 1, my_island);
    assert!(
        on_battlefield(&engine, p0, derevi_empyrial_tactician()).is_some(),
        "cast out of the command zone is the one road onto the battlefield \
         she has, and it works",
    );
}

/// Disciple of the Vault ({B}, 1/1): "Whenever an artifact is put into a
/// graveyard from the battlefield, you may have target opponent lose 1
/// life."
///
/// The printed sentence says **an** artifact and not one you control, so
/// the artifact that dies here is the *opponent's* Sol Ring, killed by a
/// Vindicate the Disciple's own controller casts. Every step is played
/// through what the engine offered: the spell off `castable`, its target
/// out of the choice that followed, then the trigger's target and the
/// trigger's "may".
///
/// Three claims are struck, and each fails somewhere else. The trigger has
/// to fire at all for an artifact nobody on this side ever controlled — a
/// filter narrowed to "an artifact you control" would leave the board
/// silent. Its target is a choice over the opponents only, so the
/// controller must not be among the player options; a Disciple that could
/// point at its own seat is a card that kills you. And a yes has to take
/// the life off the seat that was named: a player target rides in
/// `chosen_player`, which is what `PlayerRel::Chosen` reads back on
/// resolution, so a trigger that asked nobody would resolve into an empty
/// list of players and move no life total at all.
#[test]
fn an_opponents_artifact_dying_takes_a_life_from_the_targeted_opponent_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[disciple_of_the_vault(), plains(), swamp(), plains()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring =
        on_battlefield(&engine, p1, quiet_artifact()).expect("the opponent's artifact is out");
    let before = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    cast_from_hand(&mut engine, p0, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Vindicate may point at any permanent, the opponent's artifact included");

    // The artifact is dead and the trigger wants a target on the way to the
    // stack, which is the second choice this scenario reaches.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the Disciple's target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the Disciple's controller is the one asked");
    assert!(
        options.is_empty(),
        "the life loss points at a player, not at an object: {options:?}"
    );
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\" is a choice over the opponents only, so the \
         controller is not among them"
    );
    assert_eq!((min, max), (1, 1), "one opponent, and exactly one");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the one opponent at the table");

    // "You may" — answered here rather than by `pass_until`, because the
    // word is part of what the test is about.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the printed \"may\", taken");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "an artifact really went from the battlefield to a graveyard, so the \
         life below is that death and not something else on the board"
    );
    assert_eq!(
        engine.state().players[1].life,
        before.1 - 1,
        "the targeted opponent loses the 1 life, for an artifact they \
         controlled themselves"
    );
    assert_eq!(
        engine.state().players[0].life,
        before.0,
        "and the Disciple's controller pays nothing for it"
    );
}

/// Emry, Lurker of the Loch ({2}{U}, 1/2): "Affinity for artifacts. When Emry
/// enters, mill four cards. {T}: Choose target artifact card in your
/// graveyard. You may cast that card this turn."
///
/// One game, played through the engine's own offers: she is paid for, she
/// mills four, and on her controller's next turn — the first turn her `{T}` is
/// hers at all (CR 302.6) — she taps to lend one of the cards she milled the
/// permission the card prints. That permission is the point of the ability and
/// is spent the way a seat would spend it: the card in the graveyard turns up
/// in `legal.castable`, is cast for its own printed `{1}`, and **resolves onto
/// the battlefield**. Emry is the first card in the pool to point
/// `Effect::GrantFlashback` at a *permanent* card, and the half of flashback
/// she does not print — exile instead of the graveyard — must not reach her:
/// one more artifact standing on the table and one fewer card lying in the
/// graveyard is what says it did not.
///
/// The card stands at `Coverage::Partial`, and the first half of this test is
/// that claim rather than the working part. Two artifacts are out and exactly
/// `{U}` is floating, which is the price the printed affinity would have
/// charged — and the engine does not offer her. It is written to **fail** the
/// day that stops being true: when a cost reducer can count permanents this
/// assertion breaks, and flipping `Partial` to `Implemented` is what closes
/// it. Its counter-half is the two artifacts standing there and the two
/// further Islands right after — she is offered the moment her whole printed
/// `{2}{U}` is floating, so the refusal is the missing discount and not an
/// empty board or a card that cannot be cast at all.
#[test]
#[allow(clippy::too_many_lines)] // one game, from her cast to the one she pays for
fn emry_mills_four_and_taps_to_cast_one_of_them_but_never_costs_less() {
    let p0 = PlayerId::new(0);
    let rock = quiet_artifact();
    let mut engine = Duel::new(71, rock)
        .battlefield(0, &[island(), island(), island(), rock, rock])
        .hand(0, &[emry_lurker_of_the_loch()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    // The board the printed affinity would have read: two artifacts, so
    // "{1} less to cast for each artifact you control" would price her at {U}.
    let rocks = mine(&engine, p0, rock, Zone::Battlefield);
    assert_eq!(rocks.len(), 2, "two artifacts stand on the board");
    let islands = mine(&engine, p0, island(), Zone::Battlefield);
    assert_eq!(islands.len(), 3, "three Islands to tap");

    let emry = in_hand(&engine, p0, emry_lurker_of_the_loch()).expect("Emry is in hand");
    let first = islands[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: first })
        .expect("an Island taps for one blue");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&emry),
        "affinity for artifacts is not implemented, so one blue mana beside two \
         artifacts does not buy a card printed at {{2}}{{U}} — if this fires, a \
         cost reducer learned to count permanents and Emry is no longer \
         Coverage::Partial: {:?}",
        legal.castable
    );

    for &land in &islands[1..] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .expect("the other two Islands tap for one blue each");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&emry),
        "three Islands pay her printed {{2}}{{U}}, so the refusal above was the \
         price she was never given a discount on: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: emry })
        .expect("what the engine offers is payable");

    // She resolves, and the enters trigger mills four off the top. The library
    // is filled with the artifact she is about to reach for.
    pass_until(&mut engine, |e| {
        mine(e, p0, rock, Zone::Graveyard).len() >= 4 && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p0, emry_lurker_of_the_loch()).is_some(),
        "Emry landed"
    );
    assert_eq!(
        mine(&engine, p0, rock, Zone::Graveyard).len(),
        4,
        "mill four put four cards into the graveyard, and the graveyard was empty"
    );

    // Her `{T}` belongs to her controller's next turn (CR 302.6).
    let cast_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > cast_on
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });

    // `activate` takes the ability out of `legal.abilities` and panics if it
    // was never offered, so the tap below is one the engine published.
    activate(&mut engine, p0, emry_lurker_of_the_loch(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the choice of an artifact card in the graveyard, got {:?}",
            engine.pending()
        )
    };
    let rocks = mine(&engine, p0, rock, Zone::Battlefield);
    assert!(
        !options.iter().any(|id| rocks.contains(id)),
        "\"target artifact card in your graveyard\" never reaches an artifact \
         standing on the battlefield: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the four cards she milled, and nothing else: {options:?}"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("one of the cards the choice itself enumerated");
    pass_until(&mut engine, stack_is_empty);

    // The permission the whole card is for: with the mana floating, a card
    // lying in the graveyard is on the list of things this seat may cast.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&chosen),
        "the card Emry pointed at may be cast out of the graveyard: {:?}",
        legal.castable
    );

    let before = mine(&engine, p0, rock, Zone::Battlefield).len();
    engine
        .apply(p0, PlayerAction::CastSpell { card: chosen })
        .expect("its own printed {1}, paid out of the pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        mine(&engine, p0, rock, Zone::Battlefield).len(),
        before + 1,
        "the milled card was cast from the graveyard and resolved onto the \
         battlefield"
    );
    assert_eq!(
        mine(&engine, p0, rock, Zone::Graveyard).len(),
        3,
        "and it is the graveyard it left behind and not exile — Emry prints no \
         exile clause for the permission she grants to ride on"
    );
}

/// Eternal Witness ({1}{G}{G}, 2/1): "When this creature enters, you **may**
/// return target card from your graveyard to your hand."
///
/// Both halves of that sentence, because this is the first card to pair
/// `GraveyardToHand` with a target minimum of nought and the two halves are
/// two different things going right. Taken, the card the trigger named is
/// gone from the graveyard and the hand is one card larger. Declined, the
/// trigger leaves that graveyard alone: the printed "you may" is written here
/// as `TargetReq::up_to_one`, so choosing no target is how a player says no,
/// and an effect that assumed a target had been chosen would empty a
/// graveyard nobody pointed at.
#[test]
fn an_entering_witness_returns_the_card_it_named_and_nothing_when_it_declines() {
    let (mut engine, p0, buried) = a_witness_asking();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the builder stops at the target choice")
    };
    assert_eq!((min, max), (0, 1), "\"you may\" is the nought in the min");
    assert_eq!(
        options,
        vec![buried],
        "the one card in its controller's own graveyard is the one thing it \
         may be pointed at",
    );

    let held = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![buried],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, eternal_witness()).is_some(),
        "the Witness finished entering, so this is its own trigger resolving",
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .is_empty(),
        "the card it named left the graveyard",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        held + 1,
        "and arrived in hand",
    );

    // The other half of the "may": a fresh table, the same trigger, declined.
    let (mut engine, p0, buried) = a_witness_asking();
    let held = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("nought targets is a legal answer to an \"up to one\"");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, eternal_witness()).is_some(),
        "the declined trigger costs the Witness nothing",
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .contains(&buried),
        "nothing was named, so the card stayed where it lay",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        held,
        "and no hand grew",
    );
}

/// Golgari Thug ({1}{B}, 1/1): "When this creature dies, put target creature
/// card from your graveyard on top of your library." — and a printed
/// `Dredge 4` the card stands at `Coverage::Partial` for.
///
/// It is Myr Retriever's mirror, and the word that is *missing* is what makes
/// it one. The printed sentence says "target creature card", not "another",
/// so the Thug lying in the graveyard it has just fallen into is a legal
/// target for its own trigger: the trigger's targets are chosen as it is put
/// on the stack (CR 603.3d), by which time the Thug is a creature card in
/// that graveyard (CR 400.7). Both halves are struck here — the corpse is in
/// the offer, and the trigger resolves onto it and puts the Thug on top of
/// its owner's library.
///
/// The second half is what `Partial` promises. Dredge 4 replaces a draw with
/// "mill four cards, then return this card from your graveyard to your hand",
/// and nothing in the DSL says "instead of drawing", so the line is not
/// written at all. Mikokoro is activated in *response* to the Thug's own
/// trigger, which is the only moment in this game where the Thug's controller
/// draws a card with the Thug lying in their graveyard: a dredge that existed
/// would have to fire exactly there. The draw is asserted to have happened,
/// so the three assertions after it cannot pass by nobody having drawn.
#[test]
#[allow(clippy::too_many_lines)] // the replacement only shows itself against a draw that follows it in the same game
fn a_dying_thug_puts_its_own_corpse_on_top_and_replaces_no_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(0, &[golgari_thug(), mikokoro(), swamp(), swamp()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The library is Llanowar Elves, so seeding the graveyard puts a second
    // creature card there — the Thug needs something *other* than itself to
    // point at, or "its own corpse is offered too" would be the only thing
    // that could be offered and would say nothing.
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "a second creature card is waiting in the graveyard"
    );

    reach_their_main_phase(&mut engine, p1);
    let thug = on_battlefield(&engine, p0, golgari_thug()).expect("the Thug is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![thug],
            },
        )
        .expect("their removal may point at an ordinary creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    let corpse = in_graveyard(&engine, p0, golgari_thug()).expect("the Thug died");
    let other = in_graveyard(&engine, p0, quiet_creature()).expect("and it is not alone");
    assert!(
        options.contains(&corpse),
        "the printed sentence says no `another`, so the Thug is one of its own \
         trigger's legal targets: {options:?}"
    );
    assert!(
        options.contains(&other),
        "and so is the creature card that was already lying there: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .expect("its own corpse is one of the two legal targets");

    // The trigger is on the stack and the Thug is in the graveyard: the one
    // moment where a dredge 4 would have a draw to replace.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let well = on_battlefield(&engine, p0, mikokoro()).expect("Mikokoro is out");
    tap_mana_except(&mut engine, p0, well);
    let hand_before = cards_in(&engine, crate::zone::ZoneLocation::Hand(p0));
    let yard_before = cards_in(&engine, crate::zone::ZoneLocation::Graveyard(p0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 1,
            },
        )
        .expect("two Swamps pay the {2}");
    pass_until(&mut engine, |e| {
        cards_in(e, crate::zone::ZoneLocation::Hand(p0)) > hand_before
    });
    assert_eq!(
        cards_in(&engine, crate::zone::ZoneLocation::Hand(p0)),
        hand_before + 1,
        "the draw dredge would have replaced really happened"
    );
    assert_eq!(
        in_graveyard(&engine, p0, golgari_thug()),
        Some(corpse),
        "dredge 4 is not written: the Thug stayed in the graveyard instead of \
         coming back to hand off that draw"
    );
    assert_eq!(
        cards_in(&engine, crate::zone::ZoneLocation::Graveyard(p0)),
        yard_before,
        "and nothing was milled: dredge 4 would have put four cards here"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, golgari_thug()).is_none(),
        "the trigger resolved and the Thug left the graveyard"
    );
    let top = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .last()
        .expect("p0 still has a library");
    assert!(
        engine
            .state()
            .object(top)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == golgari_thug())),
        "and it is the card on top of its owner's library"
    );
}

/// Storm-Kiln Artist: "This creature gets +1/+0 for each artifact you
/// control. Magecraft — Whenever you cast or copy an instant or sorcery
/// spell, create a Treasure token."
///
/// One main phase holds every half of the card. The opponent's Sol Ring is
/// on the table before anything is cast, so the opening 2/2 is what says
/// "you control" is read and not merely "an artifact". Then a Dark Ritual is
/// cast, answered by a flashed Dualcaster Mage (CR 702.8a) whose enters
/// trigger copies that same Ritual, and answered again by an opponent's own
/// Ritual — so three instants pass through the stack and exactly one of them
/// is an instant *you cast*.
///
/// The card is `Coverage::Partial` on the "or copy" half, and this is where
/// that gap is nailed down rather than described: a copy is put onto the
/// stack and never cast (CR 707.10), no `GameEvent::SpellCast` is journalled
/// for it, and `Trigger::SpellCast` has nothing to hear. Counting the
/// Rituals on the stack is what keeps the claim honest — the walk waits for
/// two of them under p0's control at once, so a Treasure count of 1 means
/// "the copy minted none" and not "the copy was never made".
#[test]
fn a_cast_ritual_mints_a_treasure_that_grows_the_dwarf_and_a_copy_of_it_mints_none() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                storm_kiln_artist(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[dark_ritual(), dualcaster_mage()])
        .battlefield(1, &[swamp(), quiet_artifact()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    let artist = on_battlefield(&engine, p0, storm_kiln_artist()).expect("the Dwarf is deployed");
    assert_eq!(
        pt(&engine, artist),
        (2, 2),
        "the only artifact on the table is the opponent's, and `you control` does not reach it",
    );

    // Cast the Ritual: one instant, cast by the Dwarf's controller.
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // Answer it with the Mage, which has flash and copies a spell as it
    // enters. Casting a *creature* is no magecraft trigger of its own.
    let mage = in_hand(&engine, p0, dualcaster_mage()).expect("the Mage is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage })
        .unwrap();

    // The opponent answers with a Ritual of their own: a cast instant that is
    // not yours.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let their_ritual = in_hand(&engine, p1, dark_ritual()).expect("the opponent holds one too");
    engine
        .apply(p1, PlayerAction::CastSpell { card: their_ritual })
        .unwrap();

    // Their Ritual resolves, then the Mage, and its trigger asks which spell
    // to copy — yours is the only one left up there.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the copy trigger's target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![ritual], "the only spell still on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    // The copy is really made, and then everything resolves.
    pass_until(&mut engine, |e| rituals_on_the_stack(e, p0) == 2);
    pass_until(&mut engine, stack_is_empty);

    let minted = tokens_of(&engine, p0);
    assert_eq!(
        minted.len(),
        1,
        "three instants resolved and only the one you cast minted a Treasure",
    );
    let def = engine
        .state()
        .object(minted[0])
        .expect("the token is on the battlefield")
        .token
        .expect("a token knows what it is");
    assert_eq!(def.name, "Treasure", "magecraft mints a Treasure");
    assert!(
        tokens_of(&engine, p1).is_empty(),
        "the opponent's own Ritual is not a spell you cast",
    );
    assert_eq!(
        pt(&engine, artist),
        (3, 2),
        "2/2 plus the one artifact you control — the Treasure it just made",
    );
}

/// Thought Monitor ({6}{U}, a 2/2 flier): "Affinity for artifacts" and
/// "When this creature enters, draw two cards."
///
/// Both halves of its `Coverage::Partial`, because a Partial test that plays
/// only the half that works says nothing about the half that does not.
///
/// The half that works is the enters trigger: the spell resolves, the trigger
/// goes on the stack, and its controller's library is two cards shorter.
///
/// The half that does not is affinity. `FaceDef::cost_reduction` carries one
/// variant — `CostReduction::NotStartingPlayer` — and `casting::printed_reduction`
/// reads that one and nothing else, so no rule in the engine counts
/// permanents on the battlefield and this face declares no reduction at all.
/// The bill therefore stays {6}{U} with three artifacts out: six Islands
/// would pay the printed {3}{U} with room to spare, here they do not pay at
/// all, and the seventh Island is what flips the offer. That the offer gates
/// on the *quantity* of mana floating — rather than only on its colours — is
/// what `a_printed_cost_reduction_is_counted_by_the_offer_as_well` settled,
/// which is what makes one Island a discriminator instead of a coincidence.
#[test]
fn a_thought_monitor_draws_two_as_it_enters_and_three_artifacts_shorten_nothing() {
    let p0 = PlayerId::new(0);

    // The Monitor in hand over `islands` Islands and three artifacts, walked
    // to p0's own main phase with every land already tapped: `castable` is
    // read off mana that is floating, never off mana that could be. The
    // artifacts are Lightning Greaves because they make no mana — a Sol Ring
    // would pay the missing mana itself and the test would prove nothing.
    let board = |islands: usize| {
        let mut field = vec![lightning_greaves(); 3];
        field.extend(std::iter::repeat_n(island(), islands));
        let mut engine = Duel::new(77, island())
            .battlefield(0, &field)
            .hand(0, &[thought_monitor()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        tap_all_mana(&mut engine, p0);
        engine
    };

    // The declared gap, asserted rather than described: six mana and three
    // artifacts are a board the printed card casts off comfortably, and this
    // engine does not offer the spell at all.
    let six = board(6);
    let unaffordable = in_hand(&six, p0, thought_monitor()).expect("the Monitor is in hand");
    let Pending::Priority { legal, .. } = six.pending().clone() else {
        panic!("expected priority, got {:?}", six.pending())
    };
    assert!(
        !legal.castable.contains(&unaffordable),
        "three artifacts take nothing off {{6}}{{U}}, so six Islands do not \
         cast it — affinity is the gap this card declares"
    );

    // And the half that works, one Island further along.
    let mut engine = board(7);
    let spell = in_hand(&engine, p0, thought_monitor()).expect("the Monitor is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "the seventh Island is what pays for it, and nothing else changed"
    );
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = library_size(&engine, p0);

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the offer quoted a price this board pays");
    pass_until(&mut engine, stack_is_empty);

    let monitor = on_battlefield(&engine, p0, thought_monitor())
        .expect("the Monitor resolved onto the table");
    assert_eq!(pt(&engine, monitor), (2, 2), "the printed body");
    assert!(
        keywords(&engine, monitor).contains(baylee_cards_dsl::KeywordSet::FLYING),
        "and its printed flying"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"when this creature enters, draw two cards\" — two off the top",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 1,
        "the Monitor left the hand and its own trigger put two cards back",
    );
}

/// Hydroelectric Laboratory: "As this land enters, you may pay 3 life. If you
/// don't, it enters tapped. {T}: Add {U}."
///
/// Both answers are legal and they differ in exactly one bit, so they are
/// played on two boards and read against each other. Paying is the one that
/// can be pressed further: a land that stood up is a land that makes mana the
/// turn it landed, and the colour it makes is the back face's own ability
/// rather than anything the Weird on the front prints.
#[test]
fn the_laboratory_side_stands_up_for_three_life_and_comes_in_tapped_without_it() {
    let (mut engine, p0, land) = a_played_laboratory(true);
    let (calm, _, tapped_land) = a_played_laboratory(false);
    assert_eq!(
        engine
            .state()
            .object(land)
            .expect("it is on the table")
            .face_index,
        1,
        "the back face is the one that was played"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "and what it was played as is a land"
    );
    assert_eq!(
        engine.state().players[0].life,
        calm.state().players[0].life - 3,
        "three life were paid, measured against the board that declined"
    );
    assert!(!entered_tapped(&engine, land), "so it stands up");

    // "{T}: Add {U}."
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land play hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is offered its own mana ability the turn it landed"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and it is the colour the back face names"
    );

    // The other answer, on its own board.
    assert!(
        entered_tapped(&calm, tapped_land),
        "\"If you don't, it enters tapped.\""
    );
    let Pending::Priority { legal, .. } = calm.pending().clone() else {
        panic!("a land play hands priority back, got {:?}", calm.pending())
    };
    assert!(
        !legal.abilities.contains(&(tapped_land, 0)),
        "a tapped land cannot pay its own {{T}}, so the blue waits a turn"
    );
}

/// Mystic Peak, the back face: "As this land enters, you may pay 3 life. If
/// you don't, it enters tapped." and "{T}: Add {R}."
///
/// The face is reached by *playing the card as a land* (CR 712.12), and this
/// card is the shape that is offered no mode choice at all: only one of its
/// two faces is a land, so the engine plays that one the way it plays
/// Glasspool Shore — which is why the first thing asserted is the face index
/// rather than a `ChooseCastMode` the card never puts up. A front face read
/// as the land face would arrive as a 2/2 Djinn and tap for nothing.
///
/// Then both answers to the one question it does ask, because they are the
/// two different lands the same card can be: paid, it is up and makes {R}
/// the turn it lands; declined, it is down and the life total is untouched.
#[test]
fn mystic_peak_is_the_only_land_face_and_buys_itself_untapped_for_three_life() {
    let (mut engine, p0, land) = a_mystic_peak_asking(19);
    let peak = engine
        .state()
        .object(land)
        .expect("the Peak is on the battlefield while it asks");
    assert_eq!(
        peak.face_index, 1,
        "the front face is a creature, so the back is the only land face \
         there is and no mode is ever offered",
    );
    assert!(
        peak.characteristics().types.contains(TypeSet::LAND),
        "and it arrived as a land rather than as the Djinn",
    );

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Peak asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "its own controller answers");
    assert_eq!(
        prompt,
        YesNoPrompt::PayLifeOrEnterTapped { amount: 3 },
        "\"you may pay 3 life\" — three, not the two a shockland asks for",
    );

    let life = engine.state().players[0].life;
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("paying is a legal answer at twenty life");
    assert_eq!(
        engine.state().players[0].life,
        life - 3,
        "the three life is gone",
    );
    assert!(
        !is_tapped(&engine, land),
        "and that is what it bought: \"if you don't, it enters tapped\"",
    );
    assert!(
        stack_is_empty(&engine),
        "the land's printed text is two sentences and neither is a trigger: \
         the buy-back is printed on the Djinn, and the Ritual lying in the \
         graveyard was never pointed at",
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "\"{{T}}: Add {{R}}\" — an untapped Peak is offered the turn it lands",
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the ability the engine just offered");
    assert!(is_tapped(&engine, land), "the {{T}} in the cost was paid");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "and red is what came out of it",
    );

    // The other answer, on a table of its own.
    let (mut engine, p0, land) = a_mystic_peak_asking(19);
    let life = engine.state().players[0].life;
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is a legal answer too");
    assert_eq!(
        engine.state().players[0].life,
        life,
        "nothing was paid, so nothing was lost",
    );
    assert!(
        is_tapped(&engine, land),
        "\"if you don't, it enters tapped\"",
    );
    assert!(
        stack_is_empty(&engine),
        "and the declined land carries no trigger either",
    );
}

/// "Combat damage that would be dealt by creatures you control can't be
/// prevented." Maze of Ith untaps the Beast and prevents the combat damage
/// it would deal: the prevention does nothing (CR 615.12), and the opponent
/// takes 4.
#[test]
fn questing_beast_s_combat_damage_goes_through_a_maze_of_ith() {
    let p1 = PlayerId::new(1);
    let (mut engine, beast) = questing_beast_attacks(&[maze_of_ith()]);
    let mut mazed = false;
    for _ in 0..20 {
        if let Pending::Priority { player, .. } = engine.pending().clone() {
            if player == p1 {
                activate(&mut engine, p1, maze_of_ith(), 0);
                aim_at(&mut engine, p1, beast);
                mazed = true;
                break;
            }
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    assert!(
        mazed,
        "the defender gets a window after attackers are declared"
    );
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        engine.state().players[1].life,
        16,
        "4 from the Beast, unprevented"
    );
}

/// "Whenever Questing Beast deals combat damage to an opponent, it deals that
/// much damage to target planeswalker that player controls." The 4 combat
/// damage to the opponent asks for a target among that player's
/// planeswalkers — its own controller's Oko is not offered — and the Oko
/// named is dealt that much: 6 loyalty down to 2.
#[test]
fn questing_beast_deals_that_much_to_a_planeswalker_of_the_player_it_hit() {
    let p0 = PlayerId::new(0);
    let (mut engine, mine, theirs) = questing_beast_against_an_oko();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(engine.state().players[1].life, 16, "4 combat damage");
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![theirs], "a planeswalker that player controls");
    assert!(!options.contains(&mine));
    assert!(player_options.is_empty(), "a planeswalker, not a player");
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .expect("their Oko");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::Loyalty),
        2,
        "that much: 4 of its 6"
    );
    assert_eq!(counters_on(&engine, mine, CounterKind::Loyalty), 4);
    assert_eq!(
        engine.state().players[1].life,
        16,
        "no second hit on the player"
    );
}

/// Deathrite Shaman: "{B}, {T}: Exile target instant or sorcery card from a
/// graveyard. Each opponent loses 2 life."
#[test]
fn deathrite_shaman_exiles_an_instant_from_a_graveyard_and_drains_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4602, lightning_bolt())
        .battlefield(0, &[deathrite_shaman(), swamp()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, p0, 1);
    reach_main_phase(&mut engine, p0);
    let bolt = in_graveyard(&engine, p0, lightning_bolt()).expect("a Bolt in the graveyard");
    tap_all_mana_but(&mut engine, p0, Some(deathrite_shaman()));
    activate(&mut engine, p0, deathrite_shaman(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![bolt], "the only instant or sorcery card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_none(),
        "exiled"
    );
    assert_eq!(engine.state().players[1].life, 18, "each opponent loses 2");
    assert_eq!(engine.state().players[0].life, 20, "and I do not");
    let _ = p1;
}

/// A copy does not target (CR 115.1a), so shroud and hexproof do not stop it:
/// Humble Budoka (shroud) and Sylvan Caryatid (hexproof) are both on the
/// opponent's side and both are on Clone's menu, while the same two are refused
/// by a spell that does target (Unsummon). Clone copies the Caryatid and has
/// its hexproof, its 0/3 and its name.
#[test]
#[ignore = "engine bug: copy_on_enter_question asks eval::target_options, which drops shroud, hexproof and protected permanents; a copy does not target (CR 115.1a). Remove the ignore with the fix."]
fn clone_may_copy_a_creature_that_has_shroud_or_hexproof() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[clone()])
        .battlefield(1, &[humble_budoka(), sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let budoka = on_battlefield(&engine, p1, humble_budoka()).expect("shroud");
    let caryatid = on_battlefield(&engine, p1, sylvan_caryatid()).expect("hexproof");
    assert!(keywords(&engine, budoka).contains(KeywordSet::SHROUD));
    assert!(keywords(&engine, caryatid).contains(KeywordSet::HEXPROOF));

    cast_from_hand(&mut engine, p0, clone());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        panic!("expected the copy question, got {:?}", engine.pending())
    };
    assert_eq!(min, 0, "\"you may\"");
    assert!(
        options.contains(&budoka) && options.contains(&caryatid),
        "a copy is not a targeting: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![caryatid],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, clone()).expect("Clone entered");
    let c = engine.state().object(copy).unwrap().characteristics();
    assert_eq!(
        (c.power, c.toughness),
        (Some(0), Some(3)),
        "the Caryatid's body"
    );
    assert!(
        c.keywords.contains(KeywordSet::HEXPROOF),
        "and its hexproof"
    );

    // The control: a spell that does target cannot take either of them.
    let mut control = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[unsummon()])
        .battlefield(1, &[humble_budoka(), sylvan_caryatid()])
        .start();
    keep_mulligans(&mut control);
    reach_main_phase(&mut control, p0);
    let budoka = on_battlefield(&control, p1, humble_budoka()).expect("shroud");
    let caryatid = on_battlefield(&control, p1, sylvan_caryatid()).expect("hexproof");
    cast_from_hand(&mut control, p0, unsummon());
    let Pending::ChooseTargets { options, .. } = control.pending().clone() else {
        panic!("Unsummon asks for a target, got {:?}", control.pending())
    };
    assert!(
        !options.contains(&budoka) && !options.contains(&caryatid),
        "shroud and hexproof refuse a targeting spell: {options:?}"
    );
}

/// The "may": answered with no creature, Clone stays what it prints, a 0/0
/// that is put into the graveyard as a state-based action, with nothing copied.
#[test]
fn clone_declined_enters_as_a_zero_zero_and_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[clone()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, clone());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("declining the copy is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, clone()).is_none(),
        "a 0/0 does not survive"
    );
    assert!(in_graveyard(&engine, p0, clone()).is_some());
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elves are as they were"
    );
}
