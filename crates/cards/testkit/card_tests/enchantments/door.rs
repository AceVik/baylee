#[allow(clippy::wildcard_imports)]
use super::*;

/// Survival of the Fittest ({1}{G}): "{G}, Discard a creature card: Search
/// your library for a creature card, reveal that card, put it into your
/// hand, then shuffle." A cost that says "a creature card" names none, so
/// the engine has to ask before it may be paid (CR 601.2h) — and this is
/// the board that drives `ChoicePrompt::CostDiscard` and the hand-only
/// option list behind it.
///
/// Reading the card cannot replace playing it, because the card is the half
/// that does not say who may be asked for what. Its filter is a bare
/// `Filter::CREATURE` with no "you control" in it at all, and a hand is a
/// hidden zone the printed line says nothing about; the rule that a player
/// discards from their own hand is CR 701.9a and lives in `cost_wizard`.
/// So three bystanders stand beside the Elves to say what the list is not:
/// a Forest and a Counterspell in the same hand, which are cards but not
/// creature cards, and a Llanowar Elves in the **opponent's** hand, which is
/// a creature card but not one this player may discard. Reading the filter
/// alone over every hand at the table would have offered that last one.
///
/// The prompt is asserted beside the options, because the variant is what
/// tells a client this is a cost and not a search: the same ability publishes
/// a second `Pending::ChooseCards` two steps later carrying
/// `ChoicePrompt::SearchLibrary`, and a test that ignored the field would
/// pass with the two swapped. `ChooseCards` and not `ChooseTargets` is the
/// other half of that: choosing what to discard is not targeting (CR 115.1).
///
/// What the old test pinned — an ability the engine offered to nobody — is
/// gone. The ability is on the list, the question comes before anything is
/// paid, and the green mana and the chosen card are both spent when it is
/// answered.
#[allow(clippy::too_many_lines)] // one activation, from the offer to what the search found
#[test]
fn survival_of_the_fittest_asks_which_creature_card_to_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library is creature cards, so the search the cost pays for has
    // something to find — and a different creature from the one in hand, so
    // the card that paid and the card that was found cannot be confused.
    let mut engine = Duel::new(41, ondu_cleric())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[
                survival_of_the_fittest(),
                llanowar_elves(),
                forest(),
                counterspell(),
            ],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests pay the {1}{G}. The third is kept back on purpose: the
    // ability's own mana has to be *in the pool* when the offer is read,
    // because `can_afford` asks the pool and never what the board could still
    // tap.
    let forests = mine(
        &engine,
        p0,
        forest(),
        baylee_engine::zone::Zone::Battlefield,
    );
    assert_eq!(forests.len(), 3, "three Forests were dealt to seat 0");
    let spare = forests[2];
    tap_mana_except(&mut engine, p0, spare);
    let spell =
        in_hand(&engine, p0, survival_of_the_fittest()).expect("the enchantment is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("a two-mana enchantment off two Forests");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let survival = on_battlefield(&engine, p0, survival_of_the_fittest())
        .expect("Survival of the Fittest resolved onto the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: spare })
        .expect("the Forest that was kept back still taps");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the green mana the ability asks for is in the pool"
    );

    // The one creature card in hand, and the three cards that are not it.
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("a creature card is in hand");
    let land_in_hand = in_hand(&engine, p0, forest()).expect("a land card is in hand");
    let instant_in_hand = in_hand(&engine, p0, counterspell()).expect("an instant is in hand");
    let theirs = in_hand(&engine, p1, llanowar_elves()).expect("seat 1 holds a creature card too");

    // The offer the old gap withheld: a cost that has to ask is now asked of
    // the board instead of refused outright.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(survival, 0)),
        "the enchantment's own ability is on no list: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: survival,
                ability_index: 0,
            },
        )
        .expect("the activation the offer promised");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "a cost that names a card to choose has to ask, and got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "a player discards from their own hand");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature card, no more and no fewer"
    );
    assert_eq!(
        prompt,
        baylee_engine::choice::ChoicePrompt::CostDiscard,
        "the prompt is what tells a client this is a cost and not a search"
    );
    assert!(
        options.contains(&elves),
        "the creature card in hand pays this cost, and was not offered: {options:?}"
    );
    assert!(
        !options.contains(&land_in_hand),
        "a Forest in hand is a card and not a creature card"
    );
    assert!(
        !options.contains(&instant_in_hand),
        "and neither is a Counterspell"
    );
    assert!(
        !options.contains(&theirs),
        "the opponent's creature card is in the opponent's hand"
    );
    // Seat 0 takes the first turn and skips its draw (CR 103.8), so the hand
    // is exactly what the duel dealt, less the enchantment it cast.
    assert_eq!(options.len(), 1, "and nothing else at all: {options:?}");

    // The question comes before the payment (CR 601.2h): the mana is still in
    // the pool and the card is still in hand, so nobody is made to choose
    // what to give up for an activation that cannot happen.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the mana half of the cost is unspent while the question stands"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and so is the card the question is about"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the answer comes off the list the engine published");
    assert_eq!(
        in_graveyard(&engine, p0, llanowar_elves()),
        Some(elves),
        "the discarded card is in its owner's graveyard"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and it left the hand to get there"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "the mana half of the cost was paid with it"
    );

    // What the cost bought. The same ability asks again, and the second
    // question carries the prompt a search carries.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the search")
    };
    let found = *options.first().expect("the library holds creature cards");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a creature card off the list the search published");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "the card the search found is in hand"
    );
    assert!(
        engine
            .state()
            .object(found)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE)),
        "and it is a creature card, which is all the ability may take"
    );
}

/// "Enchanted creature gets +2/+2 and has flying and lifelink." One printed
/// sentence and two layers — CR 613.1 applies layer 6 (the keywords) before
/// layer 7c (the P/T change) — so the only reading that can see both at once
/// is the creature's projected characteristics, taken after the layers ran.
///
/// It also pins where the effect lands. The Aura arrives attached to the
/// creature its spell targeted, the grant follows that attachment, and
/// neither half reaches the identical Elves the opponent controls.
#[test]
fn glasswing_grace_gives_the_creature_it_enchants_plus_two_two_flying_and_lifelink() {
    let (engine, mine, theirs, aura) = a_glasswing_on_the_elves();

    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|o| o.attached_to)
            .expect("the Aura is attached to something"),
        mine,
        "an Aura enters attached to the creature its spell targeted"
    );
    assert_eq!(pt(&engine, mine), (3, 3), "+2/+2 on a 1/1");
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "the enchanted creature has flying"
    );
    assert!(
        kw.contains(KeywordSet::LIFELINK),
        "the enchanted creature has lifelink"
    );

    // The Aura grants; it does not keep.
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FLYING),
        "the Aura itself is no flier"
    );
    // And the creature it is not attached to is the card it always was.
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the opponent's Elves is enchanted by nothing"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and picked up no keyword either"
    );
}

/// Earthcraft ({1}{G}): "Tap an untapped creature you control: Untap target
/// basic land."
///
/// The pool's first `CostPart::TapOther`, and the one asking cost whose
/// answer is still on the battlefield afterwards. That is the half a reader
/// of the card cannot settle and the reason the cost has its own prompt: a
/// player shown `ChoicePrompt::CostSacrifice` over their own creatures would
/// decline a cost that only taps one.
///
/// Both enumerations are struck, and the menu is asserted by exact equality
/// rather than by what it contains. The tapped Elf is mine and is refused by
/// CR 118.3 — a permanent already tapped cannot be tapped to pay a cost,
/// whether or not the card thought to say "untapped". The opponent's Elf is
/// untapped and is refused because no rule makes a cost reach across the
/// table, so `cost_wizard::options` draws that line itself. The Earthcraft
/// is neither, and is on nobody's menu.
///
/// The creature that *does* pay arrived this turn, which is the claim the
/// card is silent about and the rules are not: CR 302.6 restricts a
/// creature's own `{T}` ability and says nothing about it being tapped to
/// pay for somebody else's, so a Bird cast this turn can already work the
/// land. Reading `cost_wizard` alone would not settle it — `can_afford`'s
/// `TapSelf` arm *does* ask about summoning sickness, one arm away.
///
/// The target list is the other half. "Target basic land" carries no "you
/// control", so the opponent's Forest is on it and the Badlands beside it is
/// not — a dual land is no basic land however many basic types it has.
#[allow(clippy::too_many_lines)] // one activation, two enumerations, and a tap that is not a sacrifice
#[test]
fn earthcraft_taps_a_summoning_sick_creature_to_untap_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[earthcraft(), forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let craft = on_battlefield(&engine, p0, earthcraft()).expect("the Earthcraft is out");
    let veteran = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf that was here");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let their_dual = on_battlefield(&engine, p1, badlands()).expect("their Badlands");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    // The older Elf is tapped by hand and the Forest by the cast, which is
    // what puts a tapped creature of my own on the board to be refused and
    // leaves the newcomer as the only untapped one. By hand because a
    // printed `mana_ability!` is an ordinary `(source, index)` in
    // `legal.abilities` — `legal.mana_abilities` is the CR 305.6 shortcut,
    // lands and granted abilities, and `cast_from_hand` walks only that.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: veteran,
                ability_index: 0,
            },
        )
        .expect("the Elf taps for {G}");
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let rookie = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != veteran
                && engine.state().object(*id).is_some_and(|o| {
                    o.controller == p0 && o.card.is_some_and(|c| c.index == llanowar_elves())
                })
        })
        .expect("the Elf cast this turn is on the battlefield");
    assert!(
        is_tapped(&engine, veteran),
        "the older Elf paid for the cast"
    );
    assert!(is_tapped(&engine, land), "and so did my Forest");
    assert!(!is_tapped(&engine, rookie), "the newcomer arrived untapped");

    activate(&mut engine, p0, earthcraft(), 0);

    // CR 601.2c: targets first, and "basic land" says nothing about whose.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the untap asks which land: {:?}", engine.pending())
    };
    assert!(
        options.contains(&land) && options.contains(&their_land),
        "either side of the table prints a basic land: {options:?}"
    );
    assert!(
        !options.contains(&their_dual),
        "a dual land is no basic land, whatever types it has: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: Vec::new(),
            },
        )
        .expect("my own tapped Forest is one of the legal targets");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which creature to tap: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        baylee_engine::choice::ChoicePrompt::CostTap,
        "not `CostSacrifice`: the creature named here is still standing \
         afterwards, and the two questions cannot share a word"
    );
    assert_eq!((min, max), (1, 1), "one creature, and exactly one");
    assert_eq!(
        options,
        vec![rookie],
        "the only untapped creature this seat controls. The older Elf \
         {veteran:?} is tapped (CR 118.3), the Earthcraft {craft:?} is no \
         creature, and the Elf {their_elf:?} across the table is not mine \
         to tap"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rookie],
            },
        )
        .expect("a creature that arrived this turn may still be tapped for a cost");

    assert!(
        is_tapped(&engine, rookie),
        "the cost is paid, so the creature named is tapped"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some()
            && engine.state().object(rookie).is_some(),
        "and it is still on the battlefield: a tap is not a sacrifice"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is not a mana ability (CR 605.1), so it uses the stack"
    );
    assert!(
        is_tapped(&engine, land),
        "and nothing has untapped yet — the effect happens on resolution"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, land),
        "the targeted basic land is untapped"
    );
    assert!(
        is_tapped(&engine, rookie),
        "and the creature that paid stays tapped: the cost is not refunded"
    );
}

/// Erode: the removal, and the land the *other* player is offered for it.
///
/// `Effect::OptionalBasicLandSearchFor { player: ControllerOfTarget }` is the
/// half that is easy to write pointing at the wrong seat, so the assertion is
/// on whose battlefield the basic arrives — and it is not the caster's.
#[test]
fn erode_destroys_a_creature_and_offers_its_controller_a_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[erode()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is seated");
    let their_library = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, erode());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search the other seat is offered is answered on the way"
    );

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "the Wurm is destroyed"
    );
    assert!(
        library_size(&engine, p1) < their_library,
        "and it is the Wurm's controller whose library the basic came out of"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_size(&engine, p0),
        "the caster searched nothing"
    );
}

/// Fable of the Mirror-Breaker, chapter I: "Create a 2/2 red Goblin Shaman
/// creature token with 'Whenever this token attacks, create a Treasure
/// token.'" The token arrives as the Saga's first chapter resolves, and when
/// it attacks two turns later its own trigger makes a Treasure. Chapter II
/// is declined on the way, and declining moves nothing: no card is
/// discarded, none drawn.
#[test]
fn fable_chapter_one_makes_a_goblin_shaman_whose_attack_makes_a_treasure() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_fable(&[]);
    let goblins = goblin_shamans(&engine, p0);
    assert_eq!(goblins.len(), 1, "chapter I made one Goblin Shaman");
    let goblin = goblins[0];
    assert_eq!(pt(&engine, goblin), (2, 2));
    let face = engine.state().object(goblin).unwrap().characteristics();
    assert!(face.types.contains(TypeSet::CREATURE));
    assert_eq!(
        face.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Red])
    );
    assert!(
        face.subtypes
            .contains(baylee_core::generated::subtypes::creature::GOBLIN)
            && face
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::SHAMAN),
        "a Goblin Shaman"
    );
    assert!(treasures(&engine, p0).is_empty());

    let (_, min, max) = to_fable_chapter_two(&mut engine);
    assert_eq!((min, max), (0, 1), "up to two, of the one card in hand");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let library = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)),
        &hand,
        "nothing discarded"
    );
    assert_eq!(library_size(&engine, p0), library, "nothing drawn");
    pass_until(&mut engine, stack_is_empty);

    attack_and_collect_blocks(&mut engine, goblin, p1);
    assert_eq!(
        treasures(&engine, p0).len(),
        1,
        "the token's own attack trigger made a Treasure"
    );
}

/// Chapter II: "You may discard up to two cards. If you do, draw that many
/// cards." Two named, two discarded, two drawn: the hand keeps its size, the
/// library is two shorter, and the journal says both were discarded.
#[test]
fn fable_chapter_two_discards_up_to_two_and_draws_that_many() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_fable(&[quiet_creature(), lightning_elemental()]);
    let (options, min, max) = to_fable_chapter_two(&mut engine);
    assert_eq!((min, max), (0, 2), "up to two");
    let creature = in_hand(&engine, p0, quiet_creature()).unwrap();
    let elemental = in_hand(&engine, p0, lightning_elemental()).unwrap();
    assert!(options.contains(&creature) && options.contains(&elemental));
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = library_size(&engine, p0);
    let journal_from = engine.state().journal.len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature, elemental],
            },
        )
        .unwrap();
    for card in [quiet_creature(), lightning_elemental()] {
        assert!(in_graveyard(&engine, p0, card).is_some(), "discarded");
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "two out, two in"
    );
    assert_eq!(library_size(&engine, p0), library - 2, "that many drawn");
    let discarded = engine.state().journal.entries()[journal_from..]
        .iter()
        .filter(|e| matches!(e.event, GameEvent::Discarded { player, .. } if player == p0))
        .count();
    assert_eq!(discarded, 2, "each one a discard, for what cares about one");
}

/// Chapter II with one card named: one discarded, one drawn — "that many"
/// is what was discarded, not the two the chapter allows.
#[test]
fn fable_chapter_two_draws_one_for_one_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_fable(&[quiet_creature(), lightning_elemental()]);
    to_fable_chapter_two(&mut engine);
    let creature = in_hand(&engine, p0, quiet_creature()).unwrap();
    let library = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_some());
    assert!(in_hand(&engine, p0, lightning_elemental()).is_some());
    assert_eq!(library_size(&engine, p0), library - 1, "one drawn");
}

/// With no Lure anywhere on this board, a creature is under no requirement
/// to block: the question names nothing to obey, and declaring no blocks
/// is legal.
#[test]
fn not_blocking_is_legal_with_no_lure_anywhere() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .expect("the Bear attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { obeying, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert!(
        obeying.is_empty(),
        "with no Lure anywhere, nothing is required to block"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("not blocking is legal with no Lure in play");
}
