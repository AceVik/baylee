//! Lands, the door `cards/lands/` puts them behind -- and the lands are
//! where most of the engine's mana arithmetic is actually played.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Abraded Bluffs: "When this land enters, it deals 1 damage to target
/// opponent." Two things are being asserted, and the card was broken on
/// both until `TargetSpec::AnyOpponent` existed. A trigger may point at a
/// *player* at all — before this, `eval::target_options` returned an empty
/// list for a player spec and CR 603.3d quietly binned the trigger — and
/// "target opponent" is a choice over the opponents only (CR 115.1), so the
/// controller must not be among the options.
#[test]
fn an_enters_trigger_can_burn_target_opponent_but_never_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(23, forest()).hand(0, &[abraded_bluffs()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == abraded_bluffs())
        })
        .expect("the land is in hand");
    let before = engine.state().players[1].life;
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the trigger: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        options.is_empty(),
        "the damage points at a player, not an object"
    );
    assert_eq!(
        player_options,
        vec![p1],
        "only the opponent is a legal target"
    );
    assert_eq!((min, max), (1, 1));

    // The controller is not on offer, and saying so anyway is refused.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .is_err(),
        "a card that says `target opponent` must not be pointable at its controller"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[1].life < before);
    assert_eq!(engine.state().players[1].life, before - 1);
}

/// Treetop Village: "{1}{G}: This land becomes a 3/3 green Ape creature
/// with trample until end of turn. It's still a land."
///
/// The transcoder writes that sentence as five continuous effects, one per
/// layer, and five plausible literals are not a working card — this is the
/// test that the composition is right: the land is a 3/3 creature *and*
/// still a land, so it can attack and still make mana.
#[test]
fn an_animated_land_becomes_a_creature_and_stays_a_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[treetop_village(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let village = on_battlefield(&engine, p0, treetop_village()).expect("village deployed");
    assert!(
        !engine
            .state()
            .object(village)
            .expect("village exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before anyone pays for it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // Index 0 is the mana ability, which `legal.abilities` lists as well;
    // index 1 is the printed "{1}{G}: … becomes a 3/3 Ape".
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, index)| *id == village && *index == 1)
        .expect("the animate ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(village)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let types = engine
        .state()
        .object(village)
        .expect("village exists")
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, village), (3, 3));
    assert!(
        engine
            .state()
            .object(village)
            .expect("village exists")
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "with trample"
    );
}

/// Rogue's Passage: "{4}, {T}: Target creature can't be blocked this turn."
///
/// `KeywordSet::UNBLOCKABLE` was read by `combat::can_block` and granted by
/// no card in the pool, so the rule had never been exercised from a card.
/// The control is inside the test rather than beside it: seat 0 attacks with
/// two creatures and only one of them was pointed at, so an empty offer —
/// which a blocker that simply could not block would also produce — is not
/// what this asserts.
#[test]
fn rogue_s_passage_takes_its_target_out_of_the_blockers_offer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                rogue_s_passage(),
                llanowar_elves(),
                ondu_cleric(),
            ],
        )
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");

    // Both of seat 0's creatures are summoning sick on turn 1, so the attack
    // is on turn 3.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::UNBLOCKABLE),
        "nothing has been activated yet"
    );

    // Four Forests: the Passage itself is not a basic land, so the CR 305.6
    // shortcut leaves it untapped to pay its own {T}.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rogue_s_passage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves), "any creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        keywords(e, elves).contains(KeywordSet::UNBLOCKABLE)
    });
    assert!(
        !keywords(&engine, cleric).contains(KeywordSet::UNBLOCKABLE),
        "the grant names the target and nothing else"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elves, Defender::Player(p1)),
                    (cleric, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    assert_eq!(blockers.len(), 1, "seat 1 has exactly one creature");
    assert_eq!(
        blockers[0].attackers,
        vec![cleric],
        "the Passage's target is not among the attackers it may be paired with"
    );
}

/// Sunken Hollow, a battle land: "This land enters tapped unless you control
/// two or more basic lands."
///
/// The condition **counts**, and what it counts is *basic* lands — two
/// things the checkland sentence next to it says neither of. The card was
/// written as a checkland ("unless you control an Island or a Swamp"), which
/// is the same answer on most boards and the wrong one on this one: two
/// Forests are two basic lands and neither is an Island.
#[test]
fn a_battle_land_counts_two_basics_of_any_kind() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        !entered_tapped(&engine, hollow),
        "two Forests are two basic lands"
    );
}

/// The other half, and it carries both bystanders the sentence needs.
///
/// One basic of your own is not two; an opponent's basics are not yours; and
/// a land that *prints* the right subtype is not basic — Irrigated Farmland
/// is a `Plains Island` and counts for nothing here, which is the half a
/// filter over subtypes gets exactly backwards.
#[test]
fn a_battle_land_counts_only_your_own_basic_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[island(), irrigated_farmland()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        entered_tapped(&engine, hollow),
        "one basic land, a nonbasic Island and two basics across the table"
    );
}

/// Deserted Beach, a slow land: "This land enters tapped unless you control
/// two or more **other** lands."
///
/// The word doing the work is "other", and the answer to it is that a
/// permanent's own enters-clause is a replacement applied on the way in, so
/// the land never counts itself. Both halves are one test because the board
/// that proves it is the same board one turn apart: the first Beach arrives
/// with an Island beside it and is the second land on the table, which is
/// two lands and still only *one* other; the second arrives with the Island
/// and the first Beach and comes in untapped. A tapped land counts — the
/// sentence asks what you control, not what is ready.
///
/// The three Forests across the table are the bystander, and they are
/// standing there for the first half, where counting them would have said
/// untapped.
#[test]
fn a_slow_land_counts_the_other_lands_and_never_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[island()])
        .battlefield(1, &[forest(), forest(), forest()])
        .hand(0, &[deserted_beach(), deserted_beach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = play_land(&mut engine, p0, deserted_beach());
    assert!(
        entered_tapped(&engine, first),
        "an Island and the Beach itself are not two other lands"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    let second = play_land(&mut engine, p0, deserted_beach());
    assert!(
        !entered_tapped(&engine, second),
        "the Island and the first Beach are two other lands"
    );
}

/// One line in `controls_at_least` skips the entering permanent, and this is
/// the list of cards that would change if it went.
///
/// Its own comment used to say the list was empty — "nothing in the pool can
/// see the difference", every enters-tapped-unless clause naming something
/// the land is not, and the one cycle that could saying "other" itself. Both
/// halves are wrong. The slow lands print `Filter::And(&[ControlledByYou,
/// LAND])` and are lands, so all ten count themselves; and the cycle the note
/// named as the safe one, Mystic Sanctuary, is a generated stub with no
/// enter-modifier at all. The line is load-bearing for ten implemented cards,
/// and the test above is what plays one.
///
/// The word the ten are missing is one the DSL *can* say — `Filter::Another`,
/// which `eval` reads as `obj.id != this` — and none of them says it, because
/// `landgen` reads "two or more other lands" into a bare count and lets the
/// engine supply the "other". That is a workable division of labour and this
/// is the fence around it: the day a filter here stops matching its own card,
/// or a new cycle starts, the split has to be looked at again rather than
/// discovered by a land that taps for one turn too few.
///
/// Asked through `eval::matches` rather than by reading the filters, because a
/// second implementation of "does this match" is exactly the thing that would
/// agree with itself and not with the engine. Both directions are asserted:
/// an unexpected land is a new cycle, a missing one is a list gone stale.
#[test]
fn the_only_lands_that_would_count_themselves_are_the_slow_ones() {
    let mut clauses = Vec::new();
    let mut behind_the_front_face = Vec::new();
    for def in baylee_cards::all() {
        for (i, face) in def.faces.iter().enumerate() {
            for modifier in face.enter_modifiers {
                let filter = match modifier {
                    baylee_cards_dsl::EnterModifier::TappedUnless(f) => *f,
                    baylee_cards_dsl::EnterModifier::TappedUnlessCount { filter, .. } => *filter,
                    _ => continue,
                };
                if i == 0 {
                    clauses.push((face.name, def.index, filter));
                } else {
                    behind_the_front_face.push(face.name);
                }
            }
        }
    }
    assert!(
        behind_the_front_face.is_empty(),
        "a back face carries an enters-tapped-unless clause, and this sweep \
         puts front faces on the table, so it was never asked about: \
         {behind_the_front_face:?}"
    );
    assert!(
        clauses.len() >= 30,
        "only {} enters-tapped-unless clauses were found; the walk is not \
         reaching the pool",
        clauses.len()
    );

    let p0 = PlayerId::new(0);
    let board: Vec<_> = clauses.iter().map(|(_, index, _)| *index).collect();
    let engine = Duel::new(114, forest()).battlefield(0, &board).start();
    let mut counts_itself = Vec::new();
    for id in engine.state().zones.list(ZoneLocation::Battlefield) {
        let obj = engine.state().object(*id).expect("on the battlefield");
        let Some(card) = obj.card else { continue };
        let Some((name, _, filter)) = clauses.iter().find(|(_, index, _)| *index == card.index)
        else {
            continue;
        };
        // `this` is the entering permanent, which is what `controls_at_least`
        // passes — so a filter that did say `Another` answers `false` here for
        // the same reason the skipped line would have made it moot.
        if crate::eval::matches(filter, engine.state(), obj, p0, obj.id) {
            counts_itself.push(*name);
        }
    }
    counts_itself.sort_unstable();
    assert_eq!(
        counts_itself, LANDS_THAT_WOULD_COUNT_THEMSELVES,
        "the set of lands whose enters-tapped-unless clause matches the land \
         itself has changed; `controls_at_least` skipping the entering \
         permanent is what makes each of them read \"other\", so a new arrival \
         needs a played test and a departure needs this list shortened"
    );
}

/// Nephalia Drownyard: "{1}{U}{B}, {T}: Target player mills three cards."
///
/// A player is the only thing this can be pointed at, and the *object* list
/// for such a spec is empty by construction — so an ability `LegalActions`
/// had just offered was refused by `apply` with "no legal targets", the
/// disagreement between two probes this engine treats as the worst kind.
/// Three implemented lands print it and all three were dead: the Drownyard,
/// Duskmantle, House of Shadow and Orzhova, the Church of Deals.
#[test]
fn a_land_that_mills_target_player_can_be_activated() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(203, forest())
        .battlefield(0, &[nephalia_drownyard(), island(), island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drownyard = on_battlefield(&engine, p0, nephalia_drownyard()).expect("the Drownyard");
    // Its own tap is part of the ability's cost, so it is the one land that
    // must not be spent on the mana.
    tap_mana_except(&mut engine, p0, drownyard);
    let before = library_size(&engine, p1);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: drownyard,
                ability_index: 1,
            },
        )
        .expect("two Islands and a Swamp pay {1}{U}{B}");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the mill asks whose library: {:?}", engine.pending())
    };
    assert!(options.is_empty(), "a seat is not an object");
    assert!(
        player_options.contains(&p1),
        "and the other seat is one of the answers"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert_eq!(
        library_size(&engine, p1),
        before - 3,
        "three cards off the top of the library that was named"
    );
}

/// Blighted Gorge: "{4}{R}, {T}, Sacrifice this land: it deals 2 damage to
/// any target" — at a table with no creature on it.
///
/// "Any target" is one set spanning objects and players (CR 115.4), and the
/// offer used to count only the objects: with an empty board the ability was
/// withheld, although a player is always there to point at. The activation
/// then had the second half of the same fault, so the two ends of this test
/// are two defects — the ability has to be offered, and it has to go
/// through.
#[test]
fn a_land_that_burns_any_target_reaches_a_face_across_an_empty_board() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(204, forest())
        .battlefield(
            0,
            &[
                blighted_gorge(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gorge = on_battlefield(&engine, p0, blighted_gorge()).expect("the Gorge");
    tap_mana_except(&mut engine, p0, gorge);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority");
    };
    assert!(
        legal.abilities.contains(&(gorge, 1)),
        "a face is a legal target even with nothing on the battlefield"
    );
    let life = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gorge,
                ability_index: 1,
            },
        )
        .expect("five Mountains pay {4}{R}");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the damage asks where: {:?}", engine.pending())
    };
    assert!(options.is_empty(), "no creature at the table");
    assert!(player_options.contains(&p1), "but both faces are there");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        life - 2,
        "two damage to the seat that was named"
    );
}

/// Fellwar Stone reads what an opponent's land "could produce" (CR 106.7),
/// and a restriction on activating is not part of that answer.
///
/// Bleachbone Verge adds {B} outright and {W} only while its controller has
/// a Plains or a Swamp. CR 106.7 asks what an ability would produce *if it
/// were to resolve*, says to ignore whether its costs could be paid, and
/// says nothing about activation restrictions — which CR 602.5 puts on
/// beginning the activation, not on the resolution. So the Verge could
/// produce {W} on a board with neither, and this test sets up exactly that
/// board: the opponent's only land is the Verge, and it is the only
/// permanent that could name a colour.
///
/// `Characteristics::from_face` collected the colours from
/// `AbilityDef::Activated { mana_ability: true }` alone, so the Verge
/// offered {B} and nothing else.
#[test]
fn a_conditional_mana_ability_still_says_what_its_land_could_produce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(733, forest())
        .battlefield(0, &[fellwar_stone()])
        .battlefield(1, &[bleachbone_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stone = on_battlefield(&engine, p0, fellwar_stone()).expect("the Stone is on the table");
    // Asked of the offer rather than assumed: `legal.mana_abilities` is the
    // intrinsic path (a basic land's own tap), and a mana ability that runs
    // an effect is offered in `legal.abilities` like any other activation —
    // it skips the stack later, in `start_activation` (CR 605.1).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (_, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == stone)
        .expect("the Stone's mana ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: stone,
                ability_index,
            },
        )
        .expect("a mana ability with no cost but the tap");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ManaColor::White),
        "the Verge could produce {{W}} however the condition stands: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black),
        "and {{B}} outright: {options:?}"
    );
}

/// Bojuka Bog: "When this land enters, exile **target player's** graveyard."
///
/// `PlayerRel::Chosen` — the seat the trigger pointed at — is the other half
/// of the relation `eval::players` cannot answer, and it fails the same
/// silent way: the loop ran over an empty list, the trigger resolved, and the
/// land was a Swamp that cost a land drop.
///
/// Both graveyards are seeded and only one is named, because a fix that
/// resolved `Chosen` as "each player" would empty the caster's own graveyard
/// too and would otherwise pass unnoticed.
#[test]
fn bojuka_bog_exiles_only_the_graveyard_it_targeted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest()).hand(0, &[bojuka_bog()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 3);
    seed_graveyard(&mut engine, p1, 3);
    let mine_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(
        mine_before, 3,
        "both graveyards start with something in them"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );

    let land = in_hand(&engine, p0, bojuka_bog()).expect("the bog is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "the enters trigger never asked for a player: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected a player choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player_options,
        vec![p0, p1],
        "\"target player\" is anyone at the table (CR 115.1), the caster included"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the bog points at the opponent");

    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty()
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        3,
        "the cards are exiled, not merely gone"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        mine_before,
        "one graveyard was named and only that one is emptied"
    );
}

/// A battle land fetched onto the battlefield still counts basics.
///
/// Prairie Stream enters tapped "unless you control two or more basic
/// lands", and one Arid Mesa activation with a single Plains out put it in
/// untapped. `EnterModifier::Tapped` on a fetched tapland was already held
/// by `s3_tests`; this is the arm beside it, which has to *count* the board
/// rather than write a status, and which no test had ever driven through a
/// search.
#[test]
fn a_fetched_battle_land_counts_the_basics_it_finds() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, prairie_stream())
        .battlefield(0, &[plains(), arid_mesa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mesa = on_battlefield(&engine, p0, arid_mesa()).expect("the fetchland is out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mesa,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the fetch's search, got {:?}", engine.pending())
    };
    let found = *options.first().expect("the library is all Prairie Stream");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();

    let obj = engine.state().object(found).expect("the land arrived");
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(
        obj.status.contains(Status::TAPPED),
        "one Plains is not two basic lands"
    );
}

/// Gaea's Cradle: "{T}: Add {G} for each creature you control."
///
/// The pool's first counted mana ability, and the reason it is worth playing
/// rather than reading: `Effect::mana_dynamic` is `AddMana` with an `Amount`
/// instead of a number, so the amount is evaluated at resolution against the
/// board — and `Filter::YOUR_CREATURE` decides whose board. No other card in
/// the pool spells `mana_dynamic`, so every one of those two joints is
/// driven here for the first time.
///
/// Both halves are struck. Two creatures of mine and one of the opponent's
/// make two green, not three and not one; a Cradle alone makes none at all,
/// which is what says the count is a count rather than a constant.
///
/// It is activated with `ActivateAbility` and not `ActivateManaAbility`:
/// `LegalActions::mana_abilities` is the CR 305.6 shortcut for a land's
/// intrinsic mana, and a *printed* mana ability is an ordinary entry in
/// `legal.abilities` however plainly it makes mana. `choice.rs` is normative
/// on that distinction.
#[test]
fn a_counted_mana_ability_counts_your_creatures_and_nobody_elses() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[gaea_s_cradle(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cradle = on_battlefield(&engine, p0, gaea_s_cradle()).expect("the Cradle is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool starts empty, so what is counted below is this ability's"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cradle,
                ability_index: 0,
            },
        )
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "two creatures of mine and one of theirs: \"each creature you \
         control\" is two"
    );
    assert_eq!(pool.total(), 2, "and it made nothing else");

    // The counterpart: the same land with nothing to count.
    let mut barren = Duel::new(61, forest())
        .battlefield(0, &[gaea_s_cradle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut barren);
    reach_main_phase(&mut barren, p0);
    let cradle = on_battlefield(&barren, p0, gaea_s_cradle()).expect("the Cradle is out");
    barren
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cradle,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        barren.state().players[0].mana_pool.total(),
        0,
        "an empty board makes no mana at all, so the amount is read and not \
         assumed"
    );
}
