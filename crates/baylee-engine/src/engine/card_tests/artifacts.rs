//! Cards whose front face is an artifact, the door `cards/artifacts/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Lightning Greaves: "Equipped creature has haste and shroud." Equipment
/// had two cards in the pool and no engine test at all, so nothing had ever
/// checked the half that matters — that the keywords land on the creature
/// the Equipment is attached to, and not on the Equipment.
#[test]
fn lightning_greaves_grants_both_keywords_to_what_it_is_attached_to() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), lightning_greaves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("greaves deployed");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "nothing is equipped yet"
    );

    reach_main_phase(&mut engine, p0);
    // Ability 1 is Equip {0}; ability 0 is the static that grants.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves], "the only creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(elves))
    });
    let kw = keywords(&engine, elves);
    assert!(
        kw.contains(KeywordSet::HASTE),
        "equipped creature has haste"
    );
    assert!(
        kw.contains(KeywordSet::SHROUD),
        "equipped creature has shroud"
    );
    assert!(
        !keywords(&engine, greaves).contains(KeywordSet::SHROUD),
        "the Equipment grants the keywords, it does not keep them"
    );
}

/// Fellwar Stone reads the colours off the lands an *opponent* controls.
/// Reflecting Pool's side of that effect had a test; this side had a card
/// (Exotic Orchard) and none — and the two differ by one comparison, so a
/// sign error there would have produced a Stone that reads your own lands
/// and passed every test in the suite.
#[test]
fn fellwar_stone_reads_the_opponents_lands_and_not_your_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(32, forest())
        .battlefield(0, &[forest(), fellwar_stone()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, fellwar_stone(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the opponent's Badlands — your own Forest is not an option"
    );
}

/// Liquimetal Coating: "{T}: **Target** permanent becomes an artifact in
/// addition to its other types until end of turn."
///
/// The same mistake Karn's `+1` made, found by the lint written for it and
/// worse: the filter reused here was `Filter::Any`, so one tap turned *every
/// permanent in the game* into an artifact — both battlefields, lands
/// included. That is the shape the owner asked about from the other side
/// ("an effect that should only appear for my field"), so the bystander here
/// is the **opponent's** land: an effect pointed at one permanent may not
/// cross the table.
#[test]
fn liquimetal_coating_plates_its_target_and_nobody_elses_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[liquimetal_coating(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let coating = on_battlefield(&engine, p0, liquimetal_coating()).expect("the coating is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("my forest");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their forest");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: coating,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target permanent\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mine)
            .is_some_and(|o| o.characteristics().types.intersects(TypeSet::ARTIFACT))
    });

    // The target is plated, and the land across the table is a plain Forest.
    assert!(
        !engine
            .state()
            .object(theirs)
            .expect("their forest is still there")
            .characteristics()
            .types
            .intersects(TypeSet::ARTIFACT),
        "the ability reached across the table and plated the opponent's land"
    );
}

/// Panharmonicon doubles a modal trigger, and each of the two chooses its
/// own mode.
///
/// The two halves of this pass in one assertion. `trigger_count` reaches
/// `ModalTriggered` because both collection loops read it through
/// `triggered_parts`, so the ability fires twice; and the mode is asked per
/// queue entry rather than per ability, so the two questions can be answered
/// differently — a Bird and a draw, off one Aether Channeler.
#[test]
fn panharmonicon_doubles_a_modal_trigger_and_each_copy_picks_its_own_mode() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(73, island())
        .battlefield(0, &[island(), island(), island(), panharmonicon()])
        .hand(0, &[aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, aether_channeler());
    let tokens_before = tokens_of(&engine, p0).len();

    // `ChooseMode` is answered by *position*, and the list holds only the
    // modes that can be chosen legally (CR 603.3c), so the position of a
    // mode is looked up rather than assumed — the bounce is on this list,
    // because Panharmonicon is itself a nonland permanent it can point at.
    let mode_at = |engine: &Engine<RegistryLookup>, mode: usize| {
        let Pending::ChooseCastMode { options, .. } = engine.pending() else {
            unreachable!("standing on the mode question")
        };
        options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == mode))
            .expect("the mode is offered")
    };

    // The first copy: a Bird.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let token_mode = mode_at(&engine, 0);
    engine
        .apply(p0, PlayerAction::ChooseMode(token_mode))
        .unwrap();
    // The second copy: a draw. Its question is a *separate* one — if the
    // mode were asked once for the ability, this would never appear.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let draw_mode = mode_at(&engine, 2);
    engine
        .apply(p0, PlayerAction::ChooseMode(draw_mode))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        tokens_before + 1,
        "one Bird, from the copy that chose the token mode",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Channeler left the hand and the draw put one card back",
    );
}

/// The other half of CR 608.2g's question, and the one with no mutant: an
/// effect that changes its target and then reads it, while the target is
/// still exactly where the resolution left it.
///
/// Inspirit Flagship Vessel stations a creature — tap it, then take its
/// power in charge counters — and the target never leaves the battlefield,
/// so the read has to be the *live* one. Nothing in the pool tells the two
/// answers apart (a tap changes no power, and these three cards are every
/// reader of `Amount::TargetPower` there is), so this test proves the branch
/// runs rather than that it is the only right one.
#[test]
fn stationing_a_creature_reads_the_power_it_still_has() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(43, plains(), &[inspirit_flagship_vessel()]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the ship is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vessel)
        .expect("the station ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(vessel)
            .map(|o| o.counters.get(CounterKind::Charge)),
        Some(2),
        "the Raptor's power on the battlefield, counter and all",
    );
}

/// A permanent that enters under a static grant is projected against it.
///
/// The projection cache is keyed on the *effect table's* generation, and a
/// permanent arriving changes no effect: so the refresh pass at the top of
/// the machine took its early exit, and the newcomer kept the cleared cache
/// `move_object` left it — which reads as the printed card. Darksteel Forge
/// says artifacts you control have indestructible, and a Sol Ring cast into
/// that board had none of it.
///
/// The board is the smaller half of the claim: the Forge is on the
/// battlefield before the game starts, so its static is registered and the
/// generation has been still ever since. Nothing but the arrival is left to
/// account for the difference.
#[test]
fn a_permanent_that_enters_under_a_static_grant_is_projected_against_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[darksteel_forge(), forest()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_artifact()).is_some()
    });
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring resolved");
    assert!(
        engine
            .state()
            .object(ring)
            .expect("the Ring is an object")
            .characteristics()
            .keywords
            .contains(KeywordSet::INDESTRUCTIBLE),
        "the Forge grants indestructible to artifacts that arrive after it too",
    );
}

/// A Spacecraft that stations to 8+ becomes a 5/5, not a corpse.
///
/// "It's an artifact creature at 8+" turns the type on, and the card def
/// carried no power or toughness at all — so the Vessel became a creature
/// with no body and the next state-based check put it into the graveyard.
/// A Spacecraft prints its numbers exactly as a Vehicle does and uses them
/// only once it is stationed.
#[test]
fn a_stationed_spacecraft_becomes_the_creature_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(0, &[inspirit_flagship_vessel()])
        .start();
    keep_mulligans(&mut engine);
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    assert!(
        !engine
            .state()
            .object(vessel)
            .expect("the Vessel is an object")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "an unstationed Spacecraft is no creature",
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, CounterKind::Charge, 8);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        on_battlefield(&engine, p0, inspirit_flagship_vessel()).is_some(),
        "a stationed Spacecraft is still on the battlefield",
    );
    let chars = engine
        .state()
        .object(vessel)
        .expect("the Vessel is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE),
        "at 8+ it is an artifact creature",
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(5), Some(5)),
        "and the body it prints is the body it gets",
    );
}

/// Ashnod's Altar ({3}): "Sacrifice a creature: Add {C}{C}."
///
/// The card stands at `Coverage::Partial`, and this is the sentence that
/// claim is made of. `cost!(Sacrifice(&Filter::YOUR_CREATURE))` says the
/// printed line exactly; what no engine path can do is suspend an activation
/// to ask *which* creature while the cost is being paid, so `can_afford`
/// refuses a filtered choice cost outright and the ability is never offered.
/// The Altar plays as though the line were not printed, which is what the
/// `Partial` promises a player.
///
/// The test is therefore that nothing is offered, and it is written to
/// **fail** the day that stops being true: when an activation can ask that
/// question, five creatures standing beside the Altar will make this break,
/// and flipping `Partial` to `Implemented` is what closes it. A card whose
/// honesty note nothing checks is a note that outlives its reason —
/// `offer_tests` says no *implemented* card may hide an unofferable ability,
/// and this is the other direction, which nothing said.
///
/// The counter-half is the board: there are creatures to feed it, so an empty
/// offer is the cost refusing and not a table with nothing on it.
#[test]
fn ashnods_altar_offers_nothing_while_a_cost_cannot_ask_which_creature() {
    let seat = PlayerId::new(0);
    let Some((engine, altars)) = arena(ashnods_altar()) else {
        panic!("the Altar is in the pool and stands on a board")
    };
    let fodder = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count();
    assert!(
        fodder >= 2,
        "the board has creatures to sacrifice, so an empty offer below is the \
         cost and not an empty table: {fodder}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the arena leaves the seat at a quiet main phase")
    };
    let offered = deeds(&legal, &altars);
    assert!(
        offered.is_empty(),
        "a sacrifice cost cannot be chosen during an activation, so the Altar \
         offers nothing at all — if this fires, `pay_cost` learned to ask and \
         Ashnod's Altar is no longer Coverage::Partial: {offered:?}"
    );
}

// oracle_id = "68e1f7e0-a9b3-437f-8086-0c0cb85f2880"
fn krark_clan_ironworks() -> baylee_core::ids::CardIndex {
    card_index("68e1f7e0-a9b3-437f-8086-0c0cb85f2880")
}

/// Krark-Clan Ironworks ({4}): "Sacrifice an artifact: Add {C}{C}."
///
/// A `Coverage::Partial` has two halves and this strikes both. The half that
/// works is the card: it is cast off five Forests and resolves onto the
/// battlefield as an artifact — which is also what makes it its own fodder,
/// since "an artifact" is `Filter::YOUR_ARTIFACT` and the Ironworks is one.
/// The half that does not is the only line it prints: no engine path can
/// suspend an activation to ask *which* artifact while the cost is being
/// paid, so `abilities::choice_cost_unpayable` puts `CostPart::Sacrifice`
/// out of `can_afford`'s reach and the ability is never offered. The
/// Ironworks plays as though the line were not printed, which is exactly
/// what the `Partial` promises a player.
///
/// This is the sibling of
/// `ashnods_altar_offers_nothing_while_a_cost_cannot_ask_which_creature`, one
/// card type up, and it is written to **fail** the day the gap closes: when
/// an activation learns to ask that question, two artifacts standing beside
/// this one will break the assertion, and flipping `Partial` to
/// `Implemented` is what closes it. A honesty note nothing checks is a note
/// that outlives its reason.
///
/// Two counter-halves keep the empty offer from being an empty table. There
/// are exactly two artifacts on the board to eat, counted rather than
/// assumed. And the Sol Ring is deliberately the one mana source left
/// untapped, so the very list that fails to name the Ironworks still names
/// *an artifact's* mana ability: the offer is alive, and what is missing
/// from it is this cost. Pressing the button by hand afterwards is refused
/// and the pool stays where it was — the ability is unreachable, not merely
/// unlisted.
#[test]
fn the_ironworks_resolves_and_then_offers_no_way_to_eat_an_artifact_for_mana() {
    let p0 = PlayerId::new(0);
    // Five Forests pay the {4} with the Sol Ring still untapped.
    let mut board = vec![forest(); 5];
    board.push(quiet_artifact());
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &board)
        .hand(0, &[krark_clan_ironworks()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Sol Ring is the fodder and the control both, so it is the one
    // source that must not be spent on the casting.
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    tap_mana_except(&mut engine, p0, rock);
    let spell = in_hand(&engine, p0, krark_clan_ironworks()).expect("the Ironworks is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("five Forests pay {4}");
    pass_until(&mut engine, stack_is_empty);

    let iron = on_battlefield(&engine, p0, krark_clan_ironworks())
        .expect("the Ironworks resolved onto the battlefield");
    let fodder = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == p0
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::ARTIFACT)
            })
        })
        .count();
    assert_eq!(
        fodder, 2,
        "the Sol Ring and — because the filter is `an artifact` — the \
         Ironworks itself are both things it could eat, so the empty offer \
         below is the cost and not an empty table"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the spell resolved, so the seat is back at a quiet priority: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and it is the seat that cast it");
    assert!(
        legal.abilities.contains(&(rock, 0)),
        "the Sol Ring was kept untapped on purpose: this list still names an \
         artifact's mana ability, so it is alive: {:?}",
        legal.abilities
    );

    let offered = deeds(&legal, &[iron]);
    assert!(
        offered.is_empty(),
        "a sacrifice cost cannot be chosen during an activation, so the \
         Ironworks offers nothing at all — if this fires, `pay_cost` learned \
         to ask and Krark-Clan Ironworks is no longer Coverage::Partial: \
         {offered:?}"
    );

    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: iron,
                    ability_index: 0,
                }
            )
            .is_err(),
        "and the ability is unreachable rather than merely unlisted: pressing \
         it is refused"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before,
        "no {{C}}{{C}} reached the pool"
    );
}

// oracle_id = "04c7f4fe-2098-4311-866d-6733c08d5178"
fn nettlecyst() -> baylee_core::ids::CardIndex {
    card_index("04c7f4fe-2098-4311-866d-6733c08d5178")
}

/// Nettlecyst is `Coverage::Partial`, and both halves of that are one
/// scenario. It is cast, and the living weapon line — "create a 0/0 black
/// Phyrexian Germ creature token, then attach this to it" — is the gap: no
/// token arrives and the Equipment enters holding nobody, so it has to be
/// equipped by hand like any other. That is the half that is written, and
/// with it the static: "equipped creature gets +1/+1 for each artifact
/// and/or enchantment you control".
///
/// Three artifacts stand on the table on purpose — Nettlecyst itself, a Sol
/// Ring under the same seat, and a Sol Ring across it. `+2/+2` is the only
/// answer that both counts the Equipment and refuses the opponent's rock:
/// `+1/+1` would mean it never counted itself (the filter says nothing about
/// `Another`), `+3/+3` that "you control" was never read. The fourth
/// artifact is cast *after* the equip, so the count is shown to be read off
/// the board rather than frozen at the moment the Equipment was attached.
#[test]
fn nettlecyst_arrives_without_its_germ_and_then_grows_with_the_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let mut board = vec![forest(); 6];
    board.extend([quiet_artifact(), llanowar_elves()]);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &board)
        .hand(0, &[nettlecyst(), quiet_artifact()])
        // A creature on the other side, so "target creature you control" has
        // something it must decline to offer.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1, holding nothing");

    // Living weapon: the half the card refuses to write.
    cast_from_hand(&mut engine, p0, nettlecyst());
    pass_until(&mut engine, stack_is_empty);
    let cyst = on_battlefield(&engine, p0, nettlecyst()).expect("the Equipment resolved");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no Germ: the living weapon line is the `Coverage::Partial` gap"
    );
    assert!(
        engine
            .state()
            .object(cyst)
            .is_some_and(|o| o.attached_to.is_none()),
        "with no Germ to attach itself to, it enters holding nobody"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Equip {2} (CR 702.6): the half that is written.
    // Ability 1 is the equip; ability 0 is the static that grows the host.
    activate(&mut engine, p0, nettlecyst(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![elves],
        "target creature *you* control — the Elves across the table are not offered"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(cyst)
            .is_some_and(|o| o.attached_to == Some(elves))
    });

    assert_eq!(
        pt(&engine, elves),
        (3, 3),
        "+1/+1 for Nettlecyst itself and +1/+1 for the Sol Ring beside it, \
         and nothing at all for the Sol Ring the opponent controls"
    );

    // And the count is a count: a fourth artifact under the same seat is a
    // third +1/+1, on a creature that was equipped two casts ago.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elves),
        (4, 4),
        "the static reads the board it is on, not the board it was equipped on"
    );
}

// oracle_id = "eb7a1f21-a66d-415b-8520-710b44890bb6"
fn simulacrum_synthesizer() -> baylee_core::ids::CardIndex {
    card_index("eb7a1f21-a66d-415b-8520-710b44890bb6")
}

/// How many artifacts `seat` controls, read after the layer system has run.
///
/// The counter-half of the Construct's own arithmetic: `ModifyPTPerCount`
/// counts the permanents the *effect's controller* controls, so the reading
/// is only worth anything with an opponent's artifacts standing on the same
/// battlefield and left out by the count rather than by the board.
fn artifacts_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::ARTIFACT)
            })
        })
        .count()
}

/// Simulacrum Synthesizer ({2}{U}): "When this artifact enters, scry 2.
/// Whenever **another** artifact you control with mana value 3 or greater
/// enters, create a 0/0 colorless Construct artifact creature token with
/// 'This token gets +1/+1 for each artifact you control.'"
///
/// Both printed sentences are played in one first main phase, off one
/// tapping of six Islands: a mana pool empties when a step or phase ends
/// (CR 500.4) and this test never leaves that phase, so the {3} left over
/// from casting the Synthesizer is what the second artifact is cast with.
///
/// The word the second sentence turns on is `another`, and it is struck
/// first: the Synthesizer is itself an artifact of mana value 3 entering
/// under its own controller, so a filter without that word would hand out
/// a Construct beside its own scry. The board is read after the entry
/// trigger has finished, and there is no token on it.
///
/// Then Chromatic Lantern, which is the {3} artifact the card is written
/// about, and the Construct that follows it is a **3/3** — the Synthesizer,
/// the Lantern, and the token itself, which is an artifact creature and so
/// counts itself. Two readings hold that number down from either side. The
/// opponent's two artifacts do not count, because the modifier counts what
/// the effect's controller controls and the card prints "each artifact
/// **you** control": three, never five. And exactly one token arrives — the
/// Construct is another artifact you control entering, but a token has no
/// mana cost, and the mana value of an object with no mana cost is 0
/// (CR 202.3a), so it is never an artifact "with mana value 3 or greater"
/// and cannot feed the ability that made it.
///
/// The scry half is asserted as a **move** and not as a question that was
/// asked: the card chosen off the top lies on the bottom afterwards, the one
/// left alone is the new top card, and the library is the length it was —
/// scry looks and reorders, and draws nothing.
#[test]
fn simulacrum_synthesizer_scries_on_arrival_and_builds_only_for_another_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[simulacrum_synthesizer(), chromatic_lantern()])
        .battlefield(1, &[quiet_artifact(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The two cards the scry is about to look at, named before anything is
    // cast. The list's last entry is the top of the library and its first is
    // the bottom — the order `Effect::Scry` reads the top `n` in, and the
    // end `ZonePosition::Bottom` writes to.
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, simulacrum_synthesizer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Synthesizer's controller does the looking");
    assert_eq!(prompt, crate::choice::ChoicePrompt::ScryBottom);
    assert_eq!(options, vec![top, second], "the top two cards, top first");
    assert_eq!(
        (min, max),
        (0, 2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![top] })
        .expect("one of the two just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");

    // `another`: a mana value 3 artifact just entered under p0's control and
    // it was the Synthesizer itself, so the second ability must not see it.
    let synthesizer = on_battlefield(&engine, p0, simulacrum_synthesizer());
    assert!(synthesizer.is_some(), "the Synthesizer resolved");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and built nothing for itself"
    );

    // {3} of the six Islands is still floating, and the Lantern is the other
    // artifact — mana value 3 exactly — that the second sentence is about.
    cast_from_hand(&mut engine, p0, chromatic_lantern());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct for the Lantern, and none for the Construct itself"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(baylee_core::types::TypeSet::ARTIFACT)
            && kinds.contains(baylee_core::types::TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        (artifacts_of(&engine, p0), artifacts_of(&engine, p1)),
        (3, 2),
        "Synthesizer, Lantern and Construct on this side; two on the other"
    );
    assert_eq!(
        pt(&engine, construct),
        (3, 3),
        "+1/+1 for each artifact *you* control: three, and never the five \
         standing on the battlefield"
    );
}

// oracle_id = "d95af032-3efd-40c7-8229-ade9d974934f"
fn u_s_s_enterprise_d() -> CardIndex {
    card_index("d95af032-3efd-40c7-8229-ade9d974934f")
}

/// The quietest seven-power body in the pool, and the reason these tests
/// reach the printed "7+" through the printed ability instead of through
/// `put_counters`.
///
/// Phyrexian Fleshgorger is a `7/5` whose menace, lifelink and ward are all
/// still an unimplemented stub, so on a battlefield it is a body and nothing
/// else — and one station of it is exactly seven charge counters, which is
/// the threshold the Spacecraft prints rather than one past it. Nothing in
/// the engine's setup path reads `coverage`, so a stub is admitted on
/// `starting_battlefield` like any other printing.
fn phyrexian_fleshgorger() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Charge counters on the Spacecraft: what Station pays in, and what both
/// the type line and the keywords key off at 7+.
#[track_caller]
fn enterprise_d_charge_counters(engine: &Engine<RegistryLookup>, ship: ObjectId) -> u16 {
    engine
        .state()
        .object(ship)
        .expect("the Spacecraft is an object")
        .counters
        .get(CounterKind::Charge)
}

/// The board every scenario starts from: the Spacecraft, a seven-power crew
/// and a one-power crew under the same seat, and a third creature across the
/// table that "another creature **you control**" has to decline.
fn an_enterprise_d_with_a_crew(
    seed: u64,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, island())
        .battlefield(
            0,
            &[
                u_s_s_enterprise_d(),
                phyrexian_fleshgorger(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: station is sorcery
    // speed, so the scenario needs p0's *own* main phase, and which seat the
    // seed put on the play decides whether a whole turn is in the way.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let ship = on_battlefield(&engine, p0, u_s_s_enterprise_d()).expect("the Spacecraft is out");
    let crew = on_battlefield(&engine, p0, phyrexian_fleshgorger()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("your own Elves are out");
    (engine, ship, crew, elves)
}

/// Whether the Spacecraft's Station ability is among the activations the
/// seat holding priority is being offered *right now*.
///
/// The Spacecraft's only activated ability is Station, so naming the object
/// is enough — the two statics behind it are never offered at all.
#[track_caller]
fn station_the_enterprise_d_is_offered(engine: &Engine<RegistryLookup>, ship: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(source, _)| *source == ship)
}

/// Stations `crew`: presses the Spacecraft's printed Station ability
/// (ability 0 — the two statics behind it are 1 and 2), aims it at `crew`,
/// lets it resolve, and hands back the options the choice enumerated.
///
/// The options are the return value because the printed cost is "Tap
/// **another** creature you control", and that word is only readable in what
/// the engine was willing to offer.
#[track_caller]
fn station_the_enterprise_d(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    crew: ObjectId,
) -> Vec<ObjectId> {
    activate(engine, seat, u_s_s_enterprise_d(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "station asks for another creature you control, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![crew],
            },
        )
        .expect("the crew it was aimed at was one of the options");
    pass_until(engine, stack_is_empty);
    options
}

/// "Station (Tap another creature you control: Put charge counters equal to
/// its power on this Spacecraft. Station only as a sorcery.)" — the half of
/// this `Coverage::Partial` that is written, and beside it the half that is
/// not.
///
/// The crew is a 7/5, so a count of seven is the only answer that reads the
/// creature's power at all: one would mean a counter per station, and five
/// that toughness was read instead. The Elves across the table are the
/// counter-half of "you control" and your own Elves are there so an empty
/// exclusion is not an empty board.
///
/// The gap is the printed trigger: "Whenever one or more charge counters are
/// put on U.S.S. Enterprise-D for the first time each turn, exile the top
/// card of your library. You may play that card this turn." No `Trigger`
/// fires on counters being put on an object and no `Effect` grants
/// permission to play a card out of exile, so the line is left off the card
/// entirely and this station must move neither the library nor exile. It is
/// asserted rather than merely noted so that the day a counter trigger
/// exists, this goes red and `Coverage::Partial` is what gets revisited.
#[test]
fn stationing_the_enterprise_d_taps_its_crew_for_that_creatures_power_and_exiles_nothing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, ship, crew, elves) = an_enterprise_d_with_a_crew(61);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        0,
        "nothing has been stationed yet"
    );
    let library = library_size(&engine, p0);
    let exiled = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    let offered = station_the_enterprise_d(&mut engine, p0, crew);
    assert!(
        offered.contains(&crew) && offered.contains(&elves),
        "both creatures under your own control are crew: {offered:?}"
    );
    assert!(
        !offered.contains(&theirs),
        "\"another creature you control\" declines the Elf across the table"
    );

    assert!(
        engine
            .state()
            .object(crew)
            .expect("the Wurm is still an object")
            .status
            .contains(Status::TAPPED),
        "stationing taps the creature it is aimed at"
    );
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        7,
        "charge counters equal to the crew's power, not one per station"
    );

    assert_eq!(
        library_size(&engine, p0),
        library,
        "the first-time-each-turn trigger is the `Coverage::Partial` gap: \
         counters went on and the top of the library stayed where it was"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled,
        "and nothing was exiled for you to play this turn"
    );
}

/// "It's an artifact creature at 7+" and "7+ | Flying, vigilance": both
/// thresholds, crossed by the printed ability rather than by the harness.
///
/// Seven counters exactly is the load-bearing number. An off-by-one in
/// either static — `at_least: 8`, or a `>` where the card says `7+` — leaves
/// a Spacecraft that is still not a creature here, and the unstationed board
/// above it is the other side of the same claim: at zero counters it is an
/// artifact with no keywords at all.
///
/// The second station is what "another" is really worth. In the test above,
/// the Spacecraft was no creature at all, so leaving it out of the options
/// proves nothing about the word; here it *is* a creature and its own
/// ability still must not offer it. That the tapped Wurm is offered a second
/// time is an observation and not an assertion — the card models the tap as
/// `Effect::TapTarget` rather than as a cost, which is a deviation from the
/// printed "Tap another creature you control:" that belongs to a different
/// test than this one.
#[test]
fn an_enterprise_d_at_seven_charge_counters_flies_with_vigilance_and_still_cannot_crew_itself() {
    let p0 = PlayerId::new(0);
    let (mut engine, ship, crew, elves) = an_enterprise_d_with_a_crew(62);
    assert!(
        !engine
            .state()
            .object(ship)
            .expect("the Spacecraft is an object")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "at zero counters it is an artifact and nothing else"
    );
    assert!(
        !keywords(&engine, ship).contains(KeywordSet::FLYING),
        "and the 7+ line grants nothing yet"
    );

    station_the_enterprise_d(&mut engine, p0, crew);
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        7,
        "the crew's seven power, which is exactly the printed threshold"
    );
    let types = engine
        .state()
        .object(ship)
        .expect("a stationed Spacecraft is still on the battlefield")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "\"It's an artifact creature at 7+\""
    );
    assert_eq!(
        pt(&engine, ship),
        (4, 5),
        "with the body the card prints, so no state-based check eats it"
    );
    let granted = keywords(&engine, ship);
    assert!(granted.contains(KeywordSet::FLYING), "7+ | Flying");
    assert!(granted.contains(KeywordSet::VIGILANCE), "7+ | vigilance");

    let offered = station_the_enterprise_d(&mut engine, p0, elves);
    assert!(
        !offered.contains(&ship),
        "\"another creature you control\" — a Spacecraft that has become a \
         creature still may not station itself"
    );
    assert!(
        offered.contains(&elves),
        "while the other creature you control is still crew: {offered:?}"
    );
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        8,
        "the Elves' one power on top of the seven already there"
    );
}

/// "Station only as a sorcery." — the third printed clause of the
/// parenthetical, and the one neither test above touches.
///
/// The negative needs its own anchor: an ability that is offered nowhere
/// would satisfy "not offered in the end step" for free. So the same board
/// is read twice — at p0's own main phase with the stack empty, where every
/// condition CR 307.1 puts on a sorcery holds, and then at p0's own **end
/// step**, where only the phase has changed. The end step rather than the
/// opponent's turn because the active player is the one guaranteed to open
/// that priority round (CR 117.3a), so the scenario never has to wait on a
/// seat the engine might have nothing to ask.
#[test]
fn the_enterprise_d_stations_only_as_a_sorcery_and_never_in_its_own_end_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, ship, ..) = an_enterprise_d_with_a_crew(63);
    assert!(
        station_the_enterprise_d_is_offered(&engine, ship),
        "at your own main phase with an empty stack, station is a sorcery you may take"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        0,
        "nothing stationed on the way, so the ability is still there to offer"
    );
    assert!(
        !station_the_enterprise_d_is_offered(&engine, ship),
        "\"Station only as a sorcery\" — your own end step is not a main phase"
    );
}

// oracle_id = "65986c1b-8e51-4604-b685-d82fa7d1263a"
fn skullclamp() -> baylee_core::ids::CardIndex {
    card_index("65986c1b-8e51-4604-b685-d82fa7d1263a")
}

/// Skullclamp: "Equipped creature gets +1/-1. Whenever equipped creature
/// dies, draw two cards. Equip {1}."
///
/// The famous play is the whole card in one move, and nothing short of
/// playing it can see either half. A 1/1 Llanowar Elves takes the clamp and
/// becomes a 2/0, which CR 704.5f puts into the graveyard before anybody
/// receives priority — so the static's `(2, 0)` is never a projection a test
/// can read, and the creature dying is the only evidence that it applied.
/// Reading the card file says the opposite of what happens: `+1/-1` looks
/// like a downgrade, not a kill.
///
/// The draw is the half nothing else in the pool reaches: no other card
/// carries `Trigger::Dies(&Filter::AttachedToBySource)`, a filter that asks
/// the *source* what it is holding about a creature that has already left
/// the battlefield. CR 603.10a is what makes that answerable — the ability
/// looks back to the game immediately before the event, when the Elves were
/// equipped — and the attachment state-based actions (CR 704.5m-p) that let
/// a hostless Equipment go are the thing the look-back has to see past.
///
/// The second Llanowar Elves is the other half of every comparison: it
/// stands beside the first, unequipped, and is a live 1/1 when the dust
/// settles. "Equipped creature" is not "creatures you control", and a static
/// that had lost its filter would have killed the pair.
#[test]
fn skullclamp_clamps_a_one_one_into_the_graveyard_and_draws_two_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                skullclamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays unequipped");
    let (host, bystander) = (elves[0], elves[1]);
    let equipment = on_battlefield(&engine, p0, skullclamp()).expect("the Equipment is out");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 while the clamp holds nobody"
    );
    assert!(
        engine
            .state()
            .object(equipment)
            .is_some_and(|o| o.attached_to.is_none()),
        "nothing is equipped yet"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    // Equip {1} (CR 702.6). The Elves are left untapped: they make mana
    // themselves, and a host that had paid for its own clamp would still
    // die, which would make the tapping impossible to read back afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability 0 is the static, 1 the death trigger, 2 the equip.
    activate(&mut engine, p0, skullclamp(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options.len(),
        2,
        "both Elves are creatures you control: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();

    // The equip resolves, the host's toughness reaches zero, and whatever
    // that death put on the stack resolves behind it.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "+1/-1 on a 1/1 is a 2/0, and CR 704.5f puts it in the graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![bystander],
        "the clamp modifies the creature it is attached to and no other"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elves nobody equipped are the 1/1 they were printed as"
    );
    assert!(
        on_battlefield(&engine, p0, skullclamp()).is_some(),
        "the Equipment outlives the host it killed"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"whenever equipped creature dies, draw two cards\" — two off the \
         top of the library. Zero here is the look-back gap: the host's \
         death (CR 704.5f) and the Equipment coming unattached (CR 704.5m-p) \
         happen in one `sba::run` pass, and the whole fixpoint runs to \
         quiescence before `collect_triggers`, so the `Trigger::Dies` arm \
         evaluates `Filter::AttachedToBySource` against an `attached_to` \
         that has already been cleared. CR 603.10a wants the value from \
         immediately before the event"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 2,
        "and the two cards are in hand — a draw that emptied the library \
         without filling the hand would satisfy the count above"
    );
}
