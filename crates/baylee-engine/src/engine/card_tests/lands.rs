//! Lands, the door `cards/lands/` puts them behind -- and the lands are
//! where most of the engine's mana arithmetic is actually played.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_cards_dsl::counters;

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

// oracle_id = "a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2"
fn academy_ruins() -> CardIndex {
    card_index("a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2")
}

/// Walks to `seat`'s **next** first main phase, across the turn in between.
///
/// Neither of the two walkers already here can do it. `walk_to_own_main`
/// answers "we are there" at once when the game is standing in that very
/// phase, and `pass_until` has no arm for the discard the opponent owes at
/// their own cleanup — seven cards kept plus the draw of their turn is eight,
/// and the walk dies on a question it cannot answer. So the one thing an
/// untap step is needed for, a land that spent its `{T}` last turn, had no
/// road to it. `answer_one` is the shared driver that does have both arms.
#[track_caller]
fn cross_into_the_next_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let from = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number > from
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the next turn");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("never reached {seat:?}'s next main phase");
}

/// Academy Ruins: "{T}: Add {C}." and "{1}{U}, {T}: Put target artifact card
/// from **your** graveyard on top of **your** library."
///
/// The word this test exists for is *your*, twice. `TargetSpec::CardInGraveyard`
/// carries a `PlayerRel`, and a relation that widened to the table would be
/// invisible in the card file — the printing and the code would agree word for
/// word while the engine offered the artifact lying in the opponent's
/// graveyard. Reading the card cannot tell the two apart; only asking the
/// engine what it offers can.
///
/// It is struck at both readers, because there are two. The offer withholds
/// an ability with no legal target, so with `{U}{U}` floating, the land
/// untapped, their Fellwar Stone in their graveyard and a Forest of my own in
/// mine, `(ruins, 1)` must not be offered at all; one artifact card of my own
/// into my graveyard and it must appear. Then the `ChooseTargets` enumeration
/// is read directly: my Greaves in it, their Stone not, and my Forest not —
/// the third being the filter rather than the relation, so a fix that widened
/// `Filter::ARTIFACT` could not pass here either.
///
/// The mana half is the other printed sentence, and it needs the untap step:
/// the `{T}` in the recursion's cost is the same tap. Which list the ability
/// lands in is asserted rather than assumed — `legal.mana_abilities` is the
/// CR 305.6 shortcut for a land's intrinsic basic-type mana and the Ruins
/// print no basic type, so their own `{T}: Add {C}` is an ordinary activation
/// in `legal.abilities`, pressed by index like any other.
#[test]
#[allow(clippy::too_many_lines)] // two printed sentences, and an untap step between them
fn academy_ruins_reach_only_the_artifacts_in_your_own_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[academy_ruins(), island(), island(), lightning_greaves()],
        )
        .battlefield(1, &[fellwar_stone()])
        .start();
    keep_mulligans(&mut engine);

    // Their artifact into their graveyard, and a card of mine into mine. The
    // library is Forests, so what seeding puts in my graveyard is a land: the
    // filter has something to reject that the relation would have allowed.
    let stone = on_battlefield(&engine, p1, fellwar_stone()).expect("their Stone is on the table");
    let state = engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, stone);
    seed_graveyard(&mut engine, p0, 1);

    reach_main_phase(&mut engine, p0);
    let ruins = on_battlefield(&engine, p0, academy_ruins()).expect("the Ruins are on the table");
    let greaves =
        on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are on the table");
    // The land's own tap is part of the recursion's cost, so it is the one
    // permanent that must not be spent on the mana.
    tap_mana_except(&mut engine, p0, ruins);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&ruins),
        "the Ruins print no basic land type, so the CR 305.6 shortcut is not \
         theirs: their mana comes from a printed ability"
    );
    assert!(
        legal.abilities.contains(&(ruins, 0)),
        "and that printed ability is offered like any other activation: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(ruins, 1)),
        "the mana is floating and the land is untapped, so the only thing \
         standing between the recursion and the offer is its target: an \
         artifact card in the *opponent's* graveyard is none, and neither is \
         the land in my own: {:?}",
        legal.abilities
    );

    // One artifact card of my own into my graveyard, and nothing else changes.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, greaves);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ruins, 1)),
        "with an artifact card of my own lying there the ability has a target \
         and is offered: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruins,
                ability_index: 1,
            },
        )
        .expect("two Islands pay the {1}{U}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the recursion asks which card: {:?}", engine.pending())
    };
    let their_stone = in_graveyard(&engine, p1, fellwar_stone()).expect("their Stone is in theirs");
    let my_forest = in_graveyard(&engine, p0, forest()).expect("a land of mine is in mine");
    assert!(
        options.contains(&greaves),
        "my own artifact card is the target: {options:?}"
    );
    assert!(
        !options.contains(&their_stone),
        "and an artifact card in the opponent's graveyard is not, because the \
         card says *your* graveyard: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "nor is a card of mine that is no artifact: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![greaves],
            },
        )
        .expect("the Greaves are one of the legal targets");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, lightning_greaves()).is_none(),
        "the ability resolved and the Greaves left the graveyard"
    );
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 still has a library");
    assert_eq!(
        top, greaves,
        "and they are the card on top of my own library"
    );
    assert_eq!(
        in_graveyard(&engine, p1, fellwar_stone()),
        Some(their_stone),
        "one graveyard was reachable and the other was never touched"
    );
    assert!(
        is_tapped(&engine, ruins),
        "the {{T}} in the cost tapped the land"
    );

    // The second reading of "on top", and the one a player sees: the untap
    // step comes, the draw comes, and what arrives is the Greaves.
    cross_into_the_next_own_main(&mut engine, p0);
    assert!(
        in_hand(&engine, p0, lightning_greaves()).is_some(),
        "the card that was put on top is the card that was drawn"
    );

    // The other printed sentence, on the land the untap step gave back.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !is_tapped(&engine, ruins),
        "the untap step gave the land back"
    );
    assert!(
        legal.abilities.contains(&(ruins, 0)),
        "and the mana ability is offered again: {:?}",
        legal.abilities
    );
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruins,
                ability_index: 0,
            },
        )
        .expect("a mana ability whose whole cost is the tap");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a fixed {{C}} asks nothing and never reaches the stack (CR 605.1): {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 1,
        "one colourless mana in the pool, which is what the land adds"
    );
    assert!(is_tapped(&engine, ruins), "and the land is tapped for it");
}

// oracle_id = "3644f316-f9a3-46c9-9b1e-747f86cf4ead"
fn buried_ruin() -> CardIndex {
    card_index("3644f316-f9a3-46c9-9b1e-747f86cf4ead")
}

/// Puts a named card out of `seat`'s hand into `seat`'s graveyard, and
/// answers with the object it became.
///
/// [`seed_graveyard`] takes whatever is on top of the library, which is the
/// filler printing and nothing else. A test that has to tell an artifact card
/// in the graveyard from a creature card beside it needs to name both, so it
/// deals them into the opening hand and buries them by name. The id is
/// handed back because an object changes id when it changes zone (CR 400.7),
/// and the id a target list is compared against has to be the one the card
/// has *in the graveyard*.
#[track_caller]
fn bury_from_hand(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> ObjectId {
    let held = in_hand(engine, seat, card).expect("the card starts in hand");
    let buried = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .move_object(
            held,
            ZoneLocation::Graveyard(seat),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    // The offer standing in `pending` was computed before this, and an
    // ability that reads a graveyard is withheld while no graveyard holds
    // what it needs.
    engine.refresh_offer();
    buried
}

/// Buried Ruin: "{2}, {T}, Sacrifice this land: Return target artifact card
/// from your graveyard to your hand."
///
/// Three clauses of that sentence are only readable by playing it. **Your**
/// graveyard: an artifact card lying in the opponent's is not an answer, and
/// a `PlayerRel` read as "any" would offer it. **Artifact** card: a creature
/// card in your own graveyard is not an answer either, and the two wrong
/// readings are different bugs, so both are on the table at once — while the
/// only artifact card in the game sits across from you the ability is
/// withheld outright, which is `ability_has_a_target` agreeing with `apply`
/// rather than the client lighting up a land that refuses the click.
///
/// And the sacrifice is a **cost**, not the effect: the land is already in
/// its owner's graveyard while the ability is still sitting on the stack, so
/// a card that spent it as part of resolving would return the artifact and
/// keep the land. The two mana are floating before any of this is asked, so
/// an ability that is not offered is not offered for want of a target.
#[test]
fn buried_ruin_pays_itself_into_the_graveyard_and_returns_only_your_own_artifact_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[buried_ruin(), forest(), forest()])
        .hand(0, &[quiet_artifact(), llanowar_elves()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let ruin = on_battlefield(&engine, p0, buried_ruin()).expect("the Ruin is on the table");

    // A creature card in my graveyard and an artifact card in theirs: every
    // near miss the target spec has to reject, and nothing it may accept.
    let elf = bury_from_hand(&mut engine, p0, llanowar_elves());
    let theirs = bury_from_hand(&mut engine, p1, quiet_artifact());
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "an artifact card really is lying in the other graveyard, so the \
         refusal below is about whose it is"
    );

    // Its own tap is part of the cost, so the Ruin is the one land that must
    // not be spent on the {2}.
    tap_mana_except(&mut engine, p0, ruin);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ruin, 1)),
        "with two mana floating and no artifact card of my own in the \
         graveyard, the ability has nothing to point at: {:?}",
        legal.abilities
    );

    let mine = bury_from_hand(&mut engine, p0, quiet_artifact());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ruin, 1)),
        "and with one it is offered: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruin,
                ability_index: 1,
            },
        )
        .expect("two Forests pay {2}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the return asks which card: {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine),
        "my own artifact card is the answer: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature card in the same graveyard is not an artifact card: \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "and an artifact card in their graveyard is not in *your* graveyard: \
         {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the one legal target");

    // CR 601.2h: the costs are paid on activation. The land is gone before
    // anything resolves.
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack and has not resolved yet"
    );
    assert!(
        on_battlefield(&engine, p0, buried_ruin()).is_none(),
        "the sacrifice is a cost, so the land left the battlefield to pay it"
    );
    assert!(
        in_graveyard(&engine, p0, buried_ruin()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard (CR 701.21a)"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_none(),
        "and it is no longer in the graveyard it came from"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "their artifact card was never touched"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "nor was the creature card lying beside mine"
    );
}

// oracle_id = "e996cd67-739c-40f4-b276-0042acf26c71"
/// Dryad Arbor, the pool's one Land Creature, under a name of its own: the
/// fixture of the same card in `instants.rs` is a sibling module's private
/// item and reaches nothing here.
fn the_land_creature() -> CardIndex {
    card_index("e996cd67-739c-40f4-b276-0042acf26c71")
}

/// Dryad Arbor prints no rules text at all — its whole card is reminder
/// text saying it is affected by summoning sickness and has "{T}: Add {G}".
///
/// So nothing in the card file can say whether the engine applies CR 302.6
/// to it: the sentence is true only if `summoning_sick` reads the
/// *projected* types, finds CREATURE on something that is also a LAND, and
/// both offer paths consult it. Two Arbors on one board answer that in one
/// priority — one seated before the game began, one played from hand this
/// turn, the same printing, both untapped, differing in nothing but when
/// they arrived.
///
/// Both lists are read, because this card is offered its {G} twice and the
/// two offers are gated in different functions: the intrinsic Forest tap
/// through `casting::can_activate_mana` into `LegalActions::mana_abilities`
/// (CR 305.6), and the printed `{T}: Add {G}` through `can_afford` into
/// `LegalActions::abilities`. A test that read one list would not see the
/// other going wrong. The pool-wide land-mana sweep skips this card by
/// name, calling it a question for a sickness test rather than a mana one;
/// this is that test.
#[test]
fn a_land_creature_played_this_turn_makes_no_mana_and_a_seated_one_does() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(0, &[the_land_creature()])
        .hand(0, &[the_land_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let preset = on_battlefield(&engine, p0, the_land_creature()).expect("the seated Arbor");
    let fresh = play_land(&mut engine, p0, the_land_creature());
    assert_ne!(preset, fresh, "two Arbors, not one object read twice");
    assert_eq!(
        all_on_battlefield(&engine, p0, the_land_creature()).len(),
        2,
        "the land drop put the second Arbor on the battlefield"
    );

    // The control, and the reason the two halves are one board: whatever
    // the engine withholds from the Arbor played this turn, it cannot be
    // withholding it for being tapped or for not being a creature.
    for id in [preset, fresh] {
        assert!(
            types(&engine, id).contains(TypeSet::CREATURE),
            "an Arbor is a creature"
        );
        assert!(
            types(&engine, id).contains(TypeSet::LAND),
            "and a land at the same time"
        );
        assert!(!is_tapped(&engine, id), "both Arbors stand untapped");
    }

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.mana_abilities.contains(&preset),
        "the seated Arbor was refused the Forest tap CR 305.6 gives it"
    );
    assert!(
        legal.abilities.contains(&(preset, 0)),
        "and the {{T}}: Add {{G}} it actually prints"
    );
    assert!(
        !legal.mana_abilities.contains(&fresh),
        "an Arbor played this turn was offered the Forest tap anyway"
    );
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == fresh),
        "an Arbor played this turn was offered its printed {{T}} anyway"
    );

    // The enumeration and the validation read one predicate, so naming the
    // action the offer withheld has to be refused as well — otherwise a
    // client could reach past the list it was given.
    assert!(
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: fresh })
            .is_err(),
        "the engine tapped a summoning-sick Arbor for its intrinsic mana"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: fresh,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the engine tapped a summoning-sick Arbor for its printed mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two refusals still made mana"
    );

    // The other half of the sentence: the same printing, seated since
    // before the turn began, pays its own {T} and adds the green.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: preset,
                ability_index: 0,
            },
        )
        .expect("the seated Arbor taps for {G}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and it is the colour the card prints"
    );
    assert!(is_tapped(&engine, preset), "paying {{T}} left it tapped");
    assert!(
        !is_tapped(&engine, fresh),
        "while the Arbor that could not be activated is still untapped"
    );
}

// oracle_id = "1861e642-21d5-4232-89f3-b5557f2946c1"
fn phyrexian_tower() -> CardIndex {
    card_index("1861e642-21d5-4232-89f3-b5557f2946c1")
}

/// Phyrexian Tower: "{T}: Add {C}." and "{T}, Sacrifice a creature: Add
/// {B}{B}."
///
/// Two printed mana abilities on one permanent, and they share a `{T}`. That
/// is what this card proves and no other board can: the second line names no
/// creature, so the engine has to stop and ask which one (CR 601.2h), and the
/// moment it is paid the first line must be gone from the offer, because the
/// Tower is tapped.
///
/// Reading the card cannot replace it. The printed sentences say nothing
/// about *whose* creature may be eaten or whether a land counts as one, and
/// the answer to both comes from the rules rather than the card: CR 701.21a
/// lets a player sacrifice only a permanent they control, so the opponent's
/// Elf is not on the list, and the Tower itself is a land and not fodder for
/// its own mouth. `Filter::YOUR_CREATURE` in the cost would be satisfied by a
/// list built any number of wrong ways; this is the list the player is handed.
///
/// The question arrives as `Pending::ChooseCards` with
/// `ChoicePrompt::CostSacrifice`, which is asserted here rather than the
/// options alone: choosing what to sacrifice is not targeting (CR 115.1), and
/// the prompt is the only thing that tells a client this is a cost being paid
/// and not a search. A test that read the list and ignored the label would
/// pass against `ChoicePrompt::Generic`.
///
/// The counter-halves, most of them the ones this test was born with. Two
/// Elves stand under the Tower, so a two-entry list is the filter answering
/// and not an empty table; the Tower is absent from `legal.mana_abilities`,
/// because it prints no basic land type and is no CR 305.6 shortcut; the
/// `{B}{B}` is read out of the pool immediately after the answer, which is
/// where a mana ability puts it (CR 605.3b), and no `{C}` is there beside it,
/// so it is the second line that was activated and not the first. Then both
/// lines are pressed by hand against the tapped Tower and both are refused,
/// with the board still at priority — nobody was shown a sacrifice question
/// for an activation that cannot happen, which is the order `start_activation`
/// promises and nothing else pins.
#[allow(clippy::too_many_lines)] // one board read end to end: the offer, the question, the spent {T}
#[test]
fn the_tower_eats_one_creature_and_the_shared_tap_closes_both_lines() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(313, swamp())
        .battlefield(0, &[phyrexian_tower(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, phyrexian_tower()).expect("the Tower is on the table");
    let fodder = mine(&engine, p0, llanowar_elves(), Zone::Battlefield);
    assert_eq!(
        fodder.len(),
        2,
        "two creatures stand beside the Tower, so the two-entry list below is \
         the filter answering and not an empty table"
    );
    let theirs = mine(&engine, p1, llanowar_elves(), Zone::Battlefield);
    assert_eq!(
        theirs.len(),
        1,
        "and the opponent has one of their own, so its absence below is a rule \
         and not a missing creature"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected a quiet main phase, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the Tower's controller who holds it");
    assert!(
        !legal.mana_abilities.contains(&tower),
        "the Tower prints no basic land type, so it is no CR 305.6 shortcut: \
         its mana belongs to `legal.abilities` and not to this list: {:?}",
        legal.mana_abilities
    );

    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == tower)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered.as_slice(),
        [0, 1],
        "both printed lines are offered: the Tower is untapped, so `{{T}}: Add \
         {{C}}` is payable, and two creatures stand under it, so `{{T}}, \
         Sacrifice a creature: Add {{B}}{{B}}` is too. `[0]` means the \
         sacrifice cost is being refused for the board instead of asked \
         about. Got: {offered:?}"
    );

    // CR 601.2h: the cost names no creature, so the player is asked which.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tower,
                ability_index: 1,
            },
        )
        .expect("the sacrifice line is offered, so it activates");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "paying `Sacrifice a creature` asks which one, and the engine is \
             at {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the question goes to the activating player");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost is not a search (CR 115.1), and the prompt is the only thing \
         that says so to a client"
    );
    assert_eq!(
        options.len(),
        2,
        "both Elves under the Tower may be eaten, and nothing else: {options:?}"
    );
    assert!(
        options.contains(&fodder[0]) && options.contains(&fodder[1]),
        "each of the controller's own creatures is on the list: {options:?}"
    );
    assert!(
        !options.contains(&tower),
        "the Tower is a land, so it is no creature to feed itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs[0]),
        "and CR 701.21a lets a player sacrifice only what they control, so the \
         opponent's Elf is not on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder[0]],
            },
        )
        .expect("the answer names a creature the engine itself offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "a mana ability resolves without the stack (CR 605.3b), so the \
         {{B}}{{B}} is in the pool the moment the cost is answered"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "and it is the second line that was activated, not the first"
    );
    assert!(
        is_tapped(&engine, tower),
        "the {{T}} half of the cost was paid"
    );
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "exactly one Elf was eaten for it"
    );
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Graveyard).len(),
        1,
        "and it is in its owner's graveyard, which is where a sacrifice puts it"
    );
    assert_eq!(
        mine(&engine, p1, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "while the opponent's creature never moved"
    );

    // The shared {T} is spent, so both lines are gone — not merely the one
    // that was activated.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a mana ability hands priority straight back, and the engine is at \
             {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and to the same player");
    let after: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == tower)
        .map(|(_, index)| *index)
        .collect();
    assert!(
        after.is_empty(),
        "both printed lines cost `{{T}}` and the Tower is tapped, so neither is \
         offered again this turn. Got: {after:?}"
    );

    for index in [0, 1] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: tower,
                        ability_index: index,
                    }
                )
                .is_err(),
            "pressing line {index} by hand is refused too, so it is unreachable \
             rather than merely unlisted"
        );
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "and the refusal of line {index} asks nothing: a sacrifice question \
             is put only after the cost is known to be payable, so nobody is \
             made to give up a creature for an activation that cannot happen"
        );
    }
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "and the surviving Elf survived the two refusals as well"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0,
        "with no {{C}} squeezed out of a tapped Tower"
    );
}

// oracle_id = "152e7e91-4eda-4e72-a9fb-bd5cb2e68239"
fn survivors_encampment() -> CardIndex {
    card_index("152e7e91-4eda-4e72-a9fb-bd5cb2e68239")
}

// oracle_id = "e6b77545-de5c-4f4a-b7ea-83498fb33ba8"
fn holdout_settlement() -> CardIndex {
    card_index("e6b77545-de5c-4f4a-b7ea-83498fb33ba8")
}

// oracle_id = "ba11a517-1dbd-4797-9f5e-46ce0f6c77c0"
fn scene_of_the_crime() -> CardIndex {
    card_index("ba11a517-1dbd-4797-9f5e-46ce0f6c77c0")
}

/// The convoke lands: "{T}, Tap an untapped creature you control: Add one
/// mana of any color."
///
/// Three cards `landgen` reads out of the printed text now that the DSL can
/// say the cost, and they are played together because what is worth proving
/// is the *shape* — one rule wrote all three, so a test of one of them is a
/// test of the rule and a difference between them would be a difference in
/// the printing rather than in the code.
///
/// Two questions on one activation, and the order is the point: CR 601.2h
/// pays the cost while the ability is being activated, and "one mana of any
/// color" is chosen when it *resolves* (CR 605.3b, immediately, without the
/// stack). So the creature is named first and the colour second, and the
/// mana that arrives is the colour the second answer named.
///
/// The land's own `{T}` is what makes the pair worth reading twice: both of
/// its abilities print one, so paying either takes the other off the offer.
#[allow(clippy::too_many_lines)] // three cards, each activated end to end
#[test]
fn a_convoke_land_taps_a_creature_of_yours_for_a_colour_of_your_choosing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let lands = [
        survivors_encampment(),
        holdout_settlement(),
        scene_of_the_crime(),
    ];
    let mut engine = Duel::new(83, forest())
        .battlefield(
            0,
            &[
                lands[0],
                lands[1],
                lands[2],
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A creature across the table: "a creature you control" is not an
        // invitation to tap somebody else's, and no rule says so — the card
        // prints it and `cost_wizard::options` is what draws the line.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    let colours = [ManaColor::Blue, ManaColor::Red, ManaColor::White];

    for (land_index, (card, colour)) in lands.iter().zip(colours).enumerate() {
        let land = on_battlefield(&engine, p0, *card).expect("the land is on the battlefield");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        // Both of the land's abilities are printed mana abilities, and
        // `legal.mana_abilities` is the CR 305.6 shortcut — lands that make
        // mana off nothing but their own tap — so the one with a cost to ask
        // about is an ordinary pair in `legal.abilities`.
        assert!(
            legal.abilities.contains(&(land, 1)),
            "land {land_index}: the convoke line is offered while a creature \
             of mine stands untapped: {:?}",
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("the cost asks which creature instead of refusing");

        let Pending::ChooseCards {
            options,
            min,
            max,
            prompt,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "land {land_index}: the cost asks which creature to tap: {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            prompt,
            crate::choice::ChoicePrompt::CostTap,
            "land {land_index}: a tap, and not a sacrifice"
        );
        assert_eq!((min, max), (1, 1), "land {land_index}: one creature");
        assert!(
            !options.contains(&theirs),
            "land {land_index}: a creature I do not control is not mine to \
             tap: {options:?}"
        );
        assert!(
            !options.contains(&land),
            "land {land_index}: and the land is no creature: {options:?}"
        );
        let victim = *options.first().expect("an untapped creature of my own");
        assert!(
            !is_tapped(&engine, victim),
            "land {land_index}: the menu holds only untapped creatures \
             (CR 118.3)"
        );

        let before = engine.state().players[0].mana_pool.available(colour);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![victim],
                },
            )
            .expect("the Elf is one of the answers the engine listed");

        // CR 605.3b: a mana ability resolves immediately, and "any color"
        // is a choice made on the way.
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "land {land_index}: any colour is a choice: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "land {land_index}: all five colours");
        engine
            .apply(p0, PlayerAction::ChooseColor(colour))
            .expect("a colour the engine offered");

        assert!(
            is_tapped(&engine, victim),
            "land {land_index}: the creature that paid is tapped"
        );
        assert!(
            engine.state().object(victim).is_some(),
            "land {land_index}: and still on the battlefield — a tap is not \
             a sacrifice"
        );
        assert!(
            is_tapped(&engine, land),
            "land {land_index}: the land spent its own {{T}} too"
        );
        assert!(
            stack_is_empty(&engine),
            "land {land_index}: a mana ability uses no stack (CR 605.3b)"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            before + 1,
            "land {land_index}: one mana of the colour that was named"
        );

        // The shared {T} is spent, so neither of the land's two lines is on
        // offer any more.
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            !legal.abilities.contains(&(land, 1)) && !legal.mana_abilities.contains(&land),
            "land {land_index}: both printed lines cost {{T}}, and it is \
             spent: {:?} / {:?}",
            legal.abilities,
            legal.mana_abilities
        );
    }
}

// oracle_id = "d8e2efe0-33a4-4303-9e83-ac42ea5df8cb"
fn vivid_crag() -> CardIndex {
    card_index("d8e2efe0-33a4-4303-9e83-ac42ea5df8cb")
}

// oracle_id = "2da7c49f-cc1e-45d9-9cbf-067e92b0daef"
fn vivid_creek() -> CardIndex {
    card_index("2da7c49f-cc1e-45d9-9cbf-067e92b0daef")
}

// oracle_id = "b7a68899-c0d3-49e0-854b-19268ae9b89d"
fn vivid_grove() -> CardIndex {
    card_index("b7a68899-c0d3-49e0-854b-19268ae9b89d")
}

// oracle_id = "20b32052-f66f-4eb8-b56e-00d531907f19"
fn vivid_marsh() -> CardIndex {
    card_index("20b32052-f66f-4eb8-b56e-00d531907f19")
}

// oracle_id = "dee99df5-628f-4a4e-a203-4dfddc927373"
fn vivid_meadow() -> CardIndex {
    card_index("dee99df5-628f-4a4e-a203-4dfddc927373")
}

// oracle_id = "9e006a4b-8dde-4416-8cb4-8401562d0fd5"
fn tendo_ice_bridge() -> CardIndex {
    card_index("9e006a4b-8dde-4416-8cb4-8401562d0fd5")
}

// oracle_id = "ea53adbe-3f9a-4847-87c7-723ac2789918"
fn mirrodin_s_core() -> CardIndex {
    card_index("ea53adbe-3f9a-4847-87c7-723ac2789918")
}

// oracle_id = "01546b7d-a233-4176-8843-d732074dc5b6"
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

/// The Vivid cycle: "This land enters tapped with two charge counters on it."
///
/// Five cards one rule wrote, played one per turn, and the reason they are
/// one test is the reason the convoke lands are: a difference between them
/// would be a difference in the printing, not in the code.
///
/// What is being proved is that **one sentence is two replacement effects**
/// (CR 614.1c). `EnterModifier` is a list rather than a shape, so "tapped"
/// and "with two charge counters" are two entries applied to the same event,
/// and a reader that took only the first would leave five ordinary taplands
/// in the pool with an ability nothing could ever afford.
#[test]
fn the_vivid_cycle_arrives_tapped_and_brings_two_counters_with_it() {
    let p0 = PlayerId::new(0);
    let cycle = [
        vivid_crag(),
        vivid_creek(),
        vivid_grove(),
        vivid_marsh(),
        vivid_meadow(),
    ];
    let mut engine = Duel::new(541, forest()).hand(0, &cycle).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for (n, card) in cycle.into_iter().enumerate() {
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "vivid land {n}: the sentence says tapped"
        );
        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            2,
            "vivid land {n}: and the same sentence says two charge counters"
        );
        cross_into_the_next_own_main(&mut engine, p0);
    }
}

/// The other half of the Vivid land, which is the counter being **spent**.
///
/// One land, three turns, and the arc is the whole point: two counters means
/// exactly two activations of the any-colour line, and the third turn is
/// where the ability stops being offered while the land's own `{T}: Add {R}`
/// stays. That last assertion is what separates "the counter ran out" from
/// "the land is tapped", which every earlier turn would have confused.
///
/// Nothing is asked of the player about the cost, and that is deliberate:
/// the counters come off the source and the source is not a choice, so
/// [`CostPart::RemoveCounterSelf`] goes straight to the colour question
/// (CR 605.3b) rather than through a chooser with one legal answer.
///
/// [`CostPart::RemoveCounterSelf`]: baylee_cards_dsl::CostPart::RemoveCounterSelf
#[test]
fn a_vivid_land_pays_a_counter_for_a_colour_it_could_not_otherwise_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(542, forest()).hand(0, &[vivid_crag()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, vivid_crag());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "the turn it arrives it is tapped, so neither line is payable: {:?}",
        legal.abilities
    );

    for turn in 0..2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(land, 1)),
            "turn {turn}: {} counter(s) left, so the any-colour line is on \
             offer: {:?}",
            counters_on(&engine, land, CounterKind::Charge),
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("a counter is there to pay with");

        // Straight to the colour: the cost asked nobody anything.
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "turn {turn}: any colour is a choice and the cost is not: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "turn {turn}: all five colours");
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .expect("a colour the engine offered");

        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            1 - turn,
            "turn {turn}: exactly one counter came off"
        );
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Blue),
            1,
            "turn {turn}: and a blue mana a Vivid Crag's own line cannot make"
        );
        assert!(
            is_tapped(&engine, land),
            "turn {turn}: the {{T}} was paid too"
        );
    }

    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "both counters are spent"
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "an untapped land with no counters cannot pay the any-colour line: \
         {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(land, 0)),
        "and its own {{T}}: Add {{R}} is untouched, which is what says the \
         refusal above is about the counter and not about the tap: {:?}",
        legal.abilities
    );
}

/// Tendo Ice Bridge: "This land enters **with** a charge counter on it."
///
/// The same sentence as the Vivid cycle's without the word "tapped", and it
/// is a separate card rather than a parameter because the difference is the
/// whole card: Tendo is usable the turn it is played, so the counter is
/// spent on that turn's colour instead of next turn's.
#[test]
fn tendo_ice_bridge_enters_untapped_and_spends_its_one_counter_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(543, forest())
        .hand(0, &[tendo_ice_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, tendo_ice_bridge());
    assert!(
        !entered_tapped(&engine, land),
        "the printing does not say tapped"
    );
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        1,
        "and it says one counter"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the counter arrived with the land and can be spent at once");
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("any colour is a choice: {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "the counter is gone"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and green came out of a land that otherwise makes {{C}}"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)) && !legal.abilities.contains(&(land, 1)),
        "untapped again, and only the line that costs no counter is offered: \
         {:?}",
        legal.abilities
    );
}

/// Mirrodin's Core, which is the only land in the pool that fills itself.
///
/// Both counter doors on one card and one turn apart, which is why it is
/// worth a test of its own: `{T}: Put a charge counter on this land` is an
/// **effect** and goes through `replacement::put_counters`, while
/// `{T}, Remove a charge counter from this land` is a **cost** and goes
/// through `replacement::remove_counters`. A card that could do only the
/// first would be a land that fills up and never spends.
#[test]
fn mirrodin_s_core_fills_itself_and_then_spends_what_it_put_on() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(544, forest())
        .hand(0, &[mirrodin_s_core()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrodin_s_core());
    assert!(!entered_tapped(&engine, land), "the Core enters untapped");
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "and empty"
    );

    // Ability 1 is not a mana ability, so it uses the stack (CR 605.1).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the Core may charge itself");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, land),
        "it charged itself with its own {{T}}"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 2,
            },
        )
        .expect("the counter it put on itself is the one it spends");
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("any colour is a choice: {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "and it is empty again"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
    );
}

/// CR 614.16 has a direction, and this is it.
///
/// A counter-doubling replacement applies to counters being **put** on a
/// permanent — the two a Vivid land enters with are exactly that (CR 614.1c),
/// so under a Doubling Season it enters with four. Nothing in Magic
/// multiplies a counter being *removed*, which is why
/// `replacement::remove_counters` takes no multiplier at all: the cost is one
/// counter under any number of Doubling Seasons.
///
/// Both halves in one game, because a removal that doubled would be invisible
/// against a land that entered with the wrong number anyway.
#[test]
fn a_doubler_doubles_the_counters_a_land_arrives_with_and_never_the_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(545, forest())
        .battlefield(0, &[doubling_season()])
        .hand(0, &[vivid_crag()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, vivid_crag());
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        4,
        "two printed counters, put on as the land enters, doubled once"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("four counters is enough for one");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        3,
        "a cost of one counter is a cost of one counter — a doubler has \
         nothing to say about a removal"
    );
}

// oracle_id = "26259c65-8f4e-42a6-b8a7-f65c36d35c4d"
fn hickory_woodlot() -> CardIndex {
    card_index("26259c65-8f4e-42a6-b8a7-f65c36d35c4d")
}

// oracle_id = "a176924c-78fc-4151-b2b5-1547b1114a40"
fn peat_bog() -> CardIndex {
    card_index("a176924c-78fc-4151-b2b5-1547b1114a40")
}

// oracle_id = "2c38f4c7-1b3f-42b4-a175-edab7acd6cc6"
fn remote_farm() -> CardIndex {
    card_index("2c38f4c7-1b3f-42b4-a175-edab7acd6cc6")
}

// oracle_id = "c8e0a1a5-8188-4677-9d8a-a18eb593343a"
fn sandstone_needle() -> CardIndex {
    card_index("c8e0a1a5-8188-4677-9d8a-a18eb593343a")
}

// oracle_id = "e4e6e796-39ce-4a63-8c61-c7c956d75d78"
fn saprazzan_skerry() -> CardIndex {
    card_index("e4e6e796-39ce-4a63-8c61-c7c956d75d78")
}

// oracle_id = "0c828f10-4775-492f-9224-1e2814ad2cad"
fn gemstone_mine() -> CardIndex {
    card_index("0c828f10-4775-492f-9224-1e2814ad2cad")
}

/// The five Mercadian Masques depletion lands: "This land enters tapped
/// with two depletion counters on it."
///
/// One sentence the Vivid cycle also prints, with one word changed, and the
/// word is the whole test. A depletion counter is not a charge counter —
/// it is `counters::DEPLETION`, an id assigned in the DSL rather than a
/// rules kind — so both are asserted on every land: two of the one and none
/// of the other. A reader that took the number out of the phrase and threw
/// the noun away would pass the first assertion and fail the second, which
/// is exactly the failure that would otherwise have shipped five lands
/// spending counters they never arrived with.
#[test]
fn the_depletion_cycle_arrives_tapped_with_counters_of_its_own_kind() {
    let p0 = PlayerId::new(0);
    let cycle = [
        hickory_woodlot(),
        peat_bog(),
        remote_farm(),
        sandstone_needle(),
        saprazzan_skerry(),
    ];
    let mut engine = Duel::new(551, forest()).hand(0, &cycle).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for (n, card) in cycle.into_iter().enumerate() {
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "depletion land {n}: the sentence says tapped"
        );
        assert_eq!(
            counters_on(&engine, land, counters::DEPLETION),
            2,
            "depletion land {n}: and the same sentence says two depletion \
             counters"
        );
        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            0,
            "depletion land {n}: depletion and charge are two counters, and \
             this is the assertion a reader that dropped the noun fails"
        );
        cross_into_the_next_own_main(&mut engine, p0);
    }
}

/// Peat Bog's whole life: "{T}, Remove a depletion counter from this land:
/// Add {B}{B}. If there are no depletion counters on this land, sacrifice
/// it."
///
/// Two mana twice and then the land is gone, which is three things at once
/// and they are three different mechanisms. The cost takes a counter off
/// (arithmetic, no chooser). The first effect makes the mana. The second
/// effect reads the counters the cost just spent and, on the second
/// activation only, sacrifices the source — an ordinary effect in the same
/// list, because that is how the card prints it, and not a state-based
/// action that would fire somewhere else entirely.
///
/// What the order of the two effects does **not** decide is the mana: the
/// pool belongs to the player and not to the land, so a sacrifice running
/// first would still leave {B}{B} behind. Gemstone Mine is where the order
/// is observable, and that is the test below.
#[test]
fn a_depletion_land_pays_twice_and_the_second_payment_kills_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(552, forest()).hand(0, &[peat_bog()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, peat_bog());
    assert!(entered_tapped(&engine, land), "it arrives tapped");

    for turn in 0..2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(land, 0)),
            "turn {turn}: {} counter(s) left, so the line is payable: {:?}",
            counters_on(&engine, land, counters::DEPLETION),
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("a counter is there to pay with");

        // {B}{B} names its colour, so nothing is asked and the whole
        // ability — cost, mana and the clause after it — is over already.
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "turn {turn}: a fixed colour asks nobody anything: {:?}",
            engine.pending()
        );
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Black),
            2,
            "turn {turn}: two black, which is what the land prints"
        );
    }

    assert!(
        in_graveyard(&engine, p0, peat_bog()).is_some(),
        "the second activation left no depletion counters, so the same \
         ability that made the mana sacrificed the land"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land),
        "and a land in a graveyard is offered nothing: {:?}",
        legal.abilities
    );
}

/// Gemstone Mine, which is the same card asked one question: "Add one mana
/// of any color."
///
/// That question is the reason it is a separate test. A colour choice
/// **suspends** the resolution (CR 605.3b resolves a mana ability without
/// the stack, but a choice still parks it in `Engine::resolution`), so the
/// sacrifice clause is on the far side of an answer the player has not
/// given yet.
///
/// The shape is not new — every pain land has it, `{T}: Add {W} or {U}.
/// This land deals 1 damage to you.` being a colour question with an effect
/// behind it — but no engine test had ever played one. So this is the first
/// test of the resume-then-trailing-effect path, and what it covers is
/// wider than the one card.
///
/// So the middle of the last activation is asserted, and it is the whole
/// point of the test: the cost has been paid (no counters left) and the
/// land is **still on the battlefield**, because the effect that kills it
/// has not run. Answer the colour and both halves land together.
#[test]
fn gemstone_mine_dies_on_the_far_side_of_the_colour_it_asks_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(553, forest()).hand(0, &[gemstone_mine()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, gemstone_mine());
    assert!(
        !entered_tapped(&engine, land),
        "Gemstone Mine prints no `tapped`, so it works the turn it arrives"
    );
    assert_eq!(
        counters_on(&engine, land, counters::MINING),
        3,
        "three mining counters, which is three activations"
    );

    for turn in 0..3u16 {
        if turn > 0 {
            cross_into_the_next_own_main(&mut engine, p0);
        }
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("a mining counter is there to pay with");

        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "turn {turn}: any colour is a choice: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "turn {turn}: all five colours");
        assert_eq!(
            counters_on(&engine, land, counters::MINING),
            2 - turn,
            "turn {turn}: the cost is paid before the question is asked"
        );
        assert!(
            in_graveyard(&engine, p0, gemstone_mine()).is_none(),
            "turn {turn}: and the clause that would sacrifice it has not run \
             yet, because the resolution is parked on this question"
        );

        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .expect("a colour the engine offered");
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            1,
            "turn {turn}: one green, from a land that prints no green symbol"
        );
    }

    assert!(
        in_graveyard(&engine, p0, gemstone_mine()).is_some(),
        "the third answer emptied the land and the clause after the mana \
         sacrificed it"
    );
}

// oracle_id = "0799df10-b489-4f79-bf98-7a0c500b46a1"
fn fountain_of_cho() -> CardIndex {
    card_index("0799df10-b489-4f79-bf98-7a0c500b46a1")
}

// oracle_id = "136596a0-b179-40be-b42d-c0b992621c95"
fn mage_ring_network() -> CardIndex {
    card_index("136596a0-b179-40be-b42d-c0b992621c95")
}

// oracle_id = "f7dda04a-c9c6-4952-9bbc-87e3c7480347"
fn saprazzan_cove() -> CardIndex {
    card_index("f7dda04a-c9c6-4952-9bbc-87e3c7480347")
}

// oracle_id = "0bbd5a04-c281-4afb-98a1-657b4eca102c"
fn subterranean_hangar() -> CardIndex {
    card_index("0bbd5a04-c281-4afb-98a1-657b4eca102c")
}

// oracle_id = "b02ab3c7-fe4a-443c-b860-ba971d3301b0"
fn mercadian_bazaar() -> CardIndex {
    card_index("b02ab3c7-fe4a-443c-b860-ba971d3301b0")
}

// oracle_id = "ccb2f92e-69c0-415c-81cd-52c384b3b233"
fn rushwood_grove() -> CardIndex {
    card_index("ccb2f92e-69c0-415c-81cd-52c384b3b233")
}

/// Activates a storage land's banking line and lets it resolve.
///
/// `{T}: Put a storage counter on this land` produces no mana, so it is not
/// a mana ability (CR 605.1a) and it uses the stack — the counter is not
/// there until it resolves. That is the reason this is a helper and not a
/// bare `apply`: a test that read the count straight after the press would
/// be reading the board before the ability had done anything, and would
/// then "prove" the storing line broken on every card that has one.
///
/// It also asserts what the press did *not* do. Banking names its own
/// number, so the engine must come straight back to priority — a
/// `ChooseNumber` here would mean the question had attached itself to the
/// permanent rather than to the cost part that announces one.
#[track_caller]
fn store_a_counter(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    land: ObjectId,
    ability_index: u32,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("an untapped land may bank a counter");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "banking a counter announces nothing: {:?}",
        engine.pending()
    );
    pass_until(engine, stack_is_empty);
}

/// Presses the storage line and answers `x`, returning the bound the engine
/// offered.
///
/// It is written as one step because the two halves are one decision: the
/// bound is the only thing the question carries, so a test that read it
/// without answering, or answered without reading it, would be asserting
/// half of what happened.
#[track_caller]
fn spend_storage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    land: ObjectId,
    ability_index: u32,
    x: u32,
) -> (u32, u32) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("the tap is the only part of this cost that can be refused");
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "`Remove any number of storage counters` names no number: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "the activating player announces the number");
    engine
        .apply(seat, PlayerAction::ChooseNumber(x))
        .expect("a number inside the offered range");
    (min, max)
}

/// Fountain of Cho: `{T}: Put a storage counter on this land.` and
/// `{T}, Remove any number of storage counters from this land: Add {W} for
/// each storage counter removed this way.`
///
/// The number in that second line is **announced as the ability is
/// activated** (CR 601.2b, reached from CR 602.2b), which is a stage the
/// engine did not have: every cost part until now was a fixed quantity a
/// card printed, so `start_activation` walked from the zone check straight
/// to targets. `CostPart::RemoveCounterSelfX` is the first part that asks
/// the player a question *before* the payment, and X is then the same X the
/// effect reads — the cost and the mana are two halves of one number.
///
/// What this test is built around is the **bound**, because the bound is the
/// whole of the legality. Zero is a legal announcement, so `can_afford` has
/// nothing to refuse and the ability is offered whatever the land carries;
/// the only wrong answer is one larger than the counters actually there.
/// So the arc is four activations of the same land across four of its
/// controller's turns, and each of them asserts the bound the engine offers:
///
/// - stored twice, `max` is 2, and 3 is refused;
/// - spend 1, and the *next* press offers 1 — which is the assertion the
///   test exists for. It fails two different ways: an `activation_x` left
///   over from the first press would skip the question entirely, and a
///   payment that never removed the counters would still offer 2;
/// - spend the last one, and the press after that offers 0, the land still
///   answering for an ability whose only legal announcement is nothing at
///   all. A zero announcement adds no mana, which is the other half of
///   "zero is legal" and the half a `1.max(x)` anywhere would break.
#[test]
#[allow(clippy::too_many_lines)] // four activations of one land, and each is an assertion
fn a_storage_land_asks_how_many_counters_and_the_bound_shrinks_with_them() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .hand(0, &[fountain_of_cho()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fountain_of_cho());
    assert!(
        entered_tapped(&engine, land),
        "Fountain of Cho prints `This land enters tapped`"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and it arrives empty: the counters are banked one turn at a time"
    );

    // Two turns of `{T}: Put a storage counter on this land.`
    for banked in 1..=2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        store_a_counter(&mut engine, p0, land, 0);
        assert_eq!(
            counters_on(&engine, land, counters::STORAGE),
            banked,
            "one counter per activation, and the land is spent for the turn"
        );
        assert!(is_tapped(&engine, land), "which is what `{{T}}` means");
    }

    // Two counters, so two is the most that may be announced.
    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the storage line is activatable");
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected a number, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "`any number` is bounded below by nothing and above by the counters \
         that are actually on the land"
    );
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(3)).is_err(),
        "three counters is a cost nothing on this land could pay"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("one of the two is an answer inside the range");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one counter removed this way is one {{W}}"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "and exactly the announced number came off — the other is still banked"
    );

    // The assertion the test is for: the bound moved with the counters.
    cross_into_the_next_own_main(&mut engine, p0);
    assert_eq!(
        spend_storage(&mut engine, p0, land, 1, 1),
        (0, 1),
        "one counter left, so one is the most the next activation may announce"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "and the mana follows the number that was just announced, not the \
         one announced last turn"
    );
    assert_eq!(counters_on(&engine, land, counters::STORAGE), 0);

    // Empty, and still a legal activation — for nothing.
    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "`any number` includes none, so there is no counter count at which \
         the ability stops being affordable: {:?}",
        legal.abilities
    );
    assert_eq!(
        spend_storage(&mut engine, p0, land, 1, 0),
        (0, 0),
        "with nothing banked, nothing is the only thing that may be announced"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "and none of a counter is none of a mana"
    );
    assert!(
        is_tapped(&engine, land),
        "the tap was still paid, whatever the announced number was"
    );
    assert!(
        in_graveyard(&engine, p0, fountain_of_cho()).is_none(),
        "and the land is still a land: unlike the depletion cycle, nothing \
         here sacrifices anything when the counters run out"
    );
}

/// Mage-Ring Network, which prints three abilities where the five Mercadian
/// Masques lands print two: `{T}: Add {C}.`, `{1}, {T}: Put a storage counter
/// on this land.` and the storage line itself. Seven of the pool's seventeen
/// counter-X lands have three ability lines — the five Time Spiral ones and
/// Crucible of the Spirit Dragon are the rest — so this is a shape rather
/// than a card; it is simply the only one of them the engine can play today.
///
/// It is the discriminating card for **where the question comes from**.
/// Fountain of Cho alone cannot tell "this ability announces a number" from
/// "this card announces a number", because every ability it prints that
/// could ask does ask. Here the same permanent carries a plain mana ability
/// and a storage line, so a question attached to the source rather than to
/// `CostPart::RemoveCounterSelfX` would fire on `{T}: Add {C}` too — and
/// that is asserted directly.
///
/// The `{1}` is the other half. A cost with a mana part *and* an announced
/// number is the shape that would break an implementation that took the X
/// question as the whole of the cost: the generic mana still has to come out
/// of the pool, and it does, one Forest's worth.
#[test]
fn only_the_storage_line_of_mage_ring_network_asks_for_a_number() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(883, forest())
        .battlefield(0, &[mage_ring_network(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let net = on_battlefield(&engine, p0, mage_ring_network()).expect("the land is on the table");

    // `{1}, {T}: Put a storage counter on this land.` — the Forest pays the
    // generic, and the land is the one thing that must not.
    tap_mana_except(&mut engine, p0, net);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest, floating"
    );
    store_a_counter(&mut engine, p0, net, 1);
    assert_eq!(counters_on(&engine, net, counters::STORAGE), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "and the {{1}} really was paid out of the pool"
    );

    // `{T}: Add {C}.` — the same permanent, a counter on it, and no question.
    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: net,
                ability_index: 0,
            },
        )
        .expect("a printed mana ability of an untapped land");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the number belongs to the cost that removes counters, not to the \
         land that has some: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "one colourless, and none of it came from the counter"
    );
    assert_eq!(
        counters_on(&engine, net, counters::STORAGE),
        1,
        "which is still sitting there untouched"
    );

    // And the storage line, which does ask.
    cross_into_the_next_own_main(&mut engine, p0);
    assert_eq!(
        spend_storage(&mut engine, p0, net, 2, 1),
        (0, 1),
        "one banked counter is a bound of one"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "`Add {{C}} for each storage counter removed this way`"
    );
    assert_eq!(counters_on(&engine, net, counters::STORAGE), 0);
}

/// The five Mercadian Masques storage lands, which are a cycle: Fountain of
/// Cho, Saprazzan Cove, Subterranean Hangar, Mercadian Bazaar and Rushwood
/// Grove print one text with one symbol changed.
///
/// So this is the reader's test rather than the engine's. The engine sees
/// `Effect::mana_dynamic(<colour>, Amount::X)` five times and cannot tell
/// which colour is right; only the printed text can, and `landgen` is what
/// read it. Five lands banked and spent in one turn each, and the pool
/// afterwards is one of every colour — a cycle member reading the wrong
/// symbol, or all five reading the first one, fails on the count.
#[test]
fn the_storage_cycle_spends_its_counter_for_the_colour_it_prints() {
    let p0 = PlayerId::new(0);
    let cycle = [
        (fountain_of_cho(), ManaColor::White),
        (saprazzan_cove(), ManaColor::Blue),
        (subterranean_hangar(), ManaColor::Black),
        (mercadian_bazaar(), ManaColor::Red),
        (rushwood_grove(), ManaColor::Green),
    ];
    let mut engine = Duel::new(887, forest())
        .battlefield(0, &cycle.map(|(card, _)| card))
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands: Vec<ObjectId> = cycle
        .iter()
        .map(|(card, _)| on_battlefield(&engine, p0, *card).expect("the land is on the table"))
        .collect();
    for land in &lands {
        store_a_counter(&mut engine, p0, *land, 0);
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 1);
    }

    cross_into_the_next_own_main(&mut engine, p0);
    for (land, (_, color)) in lands.iter().zip(cycle) {
        assert_eq!(
            spend_storage(&mut engine, p0, *land, 1, 1),
            (0, 1),
            "one counter each, so one is each land's bound"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.available(color),
            1,
            "and each of them spends it for the symbol it prints"
        );
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 0);
    }
}

// oracle_id = "2031bc31-81cc-407a-8615-29832f586bbc"
fn calciform_pools() -> CardIndex {
    card_index("2031bc31-81cc-407a-8615-29832f586bbc")
}

// oracle_id = "130a8cf5-1354-4d17-91c8-c073642eb3db"
fn dreadship_reef() -> CardIndex {
    card_index("130a8cf5-1354-4d17-91c8-c073642eb3db")
}

// oracle_id = "6f18ea44-3efa-4a45-abc6-86a0627e40f2"
fn fungal_reaches() -> CardIndex {
    card_index("6f18ea44-3efa-4a45-abc6-86a0627e40f2")
}

// oracle_id = "33587cb2-0fd3-4e4c-bc5e-e7299cc9dab5"
fn molten_slagheap() -> CardIndex {
    card_index("33587cb2-0fd3-4e4c-bc5e-e7299cc9dab5")
}

// oracle_id = "021e4165-2f02-4bd4-86ca-cb7bf4c9e23d"
fn saltcrusted_steppe() -> CardIndex {
    card_index("021e4165-2f02-4bd4-86ca-cb7bf4c9e23d")
}

// oracle_id = "d98b4250-3492-4864-9c4c-42db09b3ccd4"
fn cascading_cataracts() -> CardIndex {
    card_index("d98b4250-3492-4864-9c4c-42db09b3ccd4")
}

/// Calciform Pools: `{1}, Remove X storage counters from this land: Add X
/// mana in any combination of {W} and/or {U}.`
///
/// The Mercadian Masques cycle pays a tap and pours one colour; this one
/// pays **mana instead of the tap** and pours a *combination*, and each of
/// those is a thing no test has held yet.
///
/// - **No `{T}` in the cost.** The land is tapped for the whole of this
///   test — it spent its tap banking the counter — and spends them anyway,
///   in the same turn it banked the second one. A cost read as "the
///   announcement plus a tap" would refuse every activation here.
/// - **A pick per mana**, which the rules have no number for: "in any
///   combination" appears nowhere in the Comprehensive Rules, so it is card
///   text and the choices are made while the effect is applied like any
///   other (CR 608.2d). `combination: true` is where that lives here. Two
///   counters ask *twice*, not once, and the two answers may differ — which
///   is the whole difference from Harabaz Druid's "add X mana of any one
///   color", one pick for the whole amount. The assertion that catches a
///   reader confusing them is that the pool ends with one white *and* one
///   blue: one pick for both would make two of whichever was named.
/// - **The options are the card's own two colours**, which is where the
///   reader is struck. `colors_of` hands `ManaSource::Choice` straight back,
///   so `Pending::ChooseColor.options` is the list `landgen` emitted.
#[test]
fn a_storage_land_that_pays_mana_pours_its_counters_into_two_colours() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(911, forest())
        .battlefield(0, &[calciform_pools(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pools = on_battlefield(&engine, p0, calciform_pools()).expect("the land is on the table");

    // Two turns of `{1}, {T}: Put a storage counter on this land.`
    for banked in 1..=2u16 {
        if banked > 1 {
            cross_into_the_next_own_main(&mut engine, p0);
        }
        tap_mana_except(&mut engine, p0, pools);
        store_a_counter(&mut engine, p0, pools, 1);
        assert_eq!(counters_on(&engine, pools, counters::STORAGE), banked);
        assert!(
            is_tapped(&engine, pools),
            "which is what its `{{T}}` pays for"
        );
    }

    // The same turn the second counter was banked in, with the land tapped.
    assert_eq!(
        spend_storage(&mut engine, p0, pools, 2, 2),
        (0, 2),
        "two counters banked, two announceable — and the tap the banking \
         line spent is not part of this cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "three Forests paid two {{1}}s, and the third is still floating"
    );

    // Two picks, and they may differ.
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("a combination asks a colour: {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "and the colours offered are the two the card prints, in its order"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("a colour the engine offered");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "`in any combination` is one pick per mana, so the second is \
             still to come: {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![ManaColor::White, ManaColor::Blue]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("and the second answer need not be the first");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(ManaColor::White),
            pool.available(ManaColor::Blue)
        ),
        (1, 1),
        "one of each: a reader that took this for `any one color` would \
         have made two of whichever was named first"
    );
    assert_eq!(counters_on(&engine, pools, counters::STORAGE), 0);
}

/// Cascading Cataracts: `{5}, {T}: Add five mana in any combination of
/// colors.`
///
/// The fixed-number form of the same sentence, and the one that says the
/// number is not the counter-X machinery wearing a hat: nothing announces
/// anything here, the cost has no counter in it at all, and five picks still
/// come out — `Amount::Fixed(5)` with `combination: true`.
///
/// Five colours offered rather than two, which is the other half of the
/// reading: `ALL_MANA_COLORS` against the `and/or` pair above.
#[test]
fn cascading_cataracts_asks_five_times_and_offers_every_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(913, forest())
        .battlefield(
            0,
            &[
                cascading_cataracts(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land =
        on_battlefield(&engine, p0, cascading_cataracts()).expect("the land is on the table");

    tap_mana_except(&mut engine, p0, land);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("five Forests pay the {5}");
    assert!(
        matches!(engine.pending(), Pending::ChooseColor { .. }),
        "no counter is announced here, so the colour is the first question: \
         {:?}",
        engine.pending()
    );

    for (i, color) in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]
    .into_iter()
    .enumerate()
    {
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("pick {i} of five: {:?}", engine.pending())
        };
        assert_eq!(
            options.len(),
            5,
            "pick {i}: `any combination of colors` is all five, every time"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(color))
            .expect("a colour the engine offered");
    }

    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "five picks and no sixth: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(
            pool.available(color),
            1,
            "five mana, one of each, which is what five separate picks buys"
        );
    }
}

/// The five Time Spiral storage lands, which print one text with one pair of
/// symbols changed.
///
/// The engine sees `Effect::mana_combination(&[_, _], Amount::X)` five times
/// and cannot tell a right pair from a wrong one; `Pending::ChooseColor`'s
/// options are `landgen`'s emitted list handed straight back, so this is the
/// reader on trial. Two lands sharing a pair, or all five reading the first
/// one, fails here.
#[test]
fn the_time_spiral_storage_cycle_offers_the_pair_each_land_prints() {
    let p0 = PlayerId::new(0);
    let cycle = [
        (calciform_pools(), [ManaColor::White, ManaColor::Blue]),
        (dreadship_reef(), [ManaColor::Blue, ManaColor::Black]),
        (molten_slagheap(), [ManaColor::Black, ManaColor::Red]),
        (fungal_reaches(), [ManaColor::Red, ManaColor::Green]),
        (saltcrusted_steppe(), [ManaColor::Green, ManaColor::White]),
    ];
    let mut board: Vec<CardIndex> = cycle.iter().map(|(card, _)| *card).collect();
    board.extend(std::iter::repeat_n(forest(), 5));
    let mut engine = Duel::new(917, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands: Vec<ObjectId> = cycle
        .iter()
        .map(|(card, _)| on_battlefield(&engine, p0, *card).expect("the land is on the table"))
        .collect();

    // Five Forests bank five counters, one per land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    for land in &lands {
        store_a_counter(&mut engine, p0, *land, 1);
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 1);
    }

    // And the next turn spends them, five more Forests paying the five {1}s.
    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    for (land, (_, pair)) in lands.iter().zip(cycle) {
        assert_eq!(
            spend_storage(&mut engine, p0, *land, 2, 1),
            (0, 1),
            "one counter each, so one is each land's bound"
        );
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("one counter buys one pick: {:?}", engine.pending())
        };
        assert_eq!(
            options,
            pair.to_vec(),
            "each land offers the pair it prints and no other"
        );
        // The delta and not the total, because the `{1}` of the *next*
        // activation is paid out of the same pool and may well be paid with
        // the mana this one just made — which is a legal thing for a player
        // to do and would make a sweep at the end read the wrong number for
        // a reason that has nothing to do with the cycle.
        let before = engine.state().players[0].mana_pool.available(pair[1]);
        engine
            .apply(p0, PlayerAction::ChooseColor(pair[1]))
            .expect("a colour the engine offered");
        assert_eq!(
            engine.state().players[0].mana_pool.available(pair[1]),
            before + 1,
            "one counter, one mana, of the colour that was picked"
        );
        assert_eq!(counters_on(&engine, *land, counters::STORAGE), 0);
    }
}

// oracle_id = "9f12bf9a-6e1a-4377-b4af-e8cabd3ee58a"
fn deserted_temple() -> CardIndex {
    card_index("9f12bf9a-6e1a-4377-b4af-e8cabd3ee58a")
}

/// Deserted Temple: `{1}, {T}: Untap target land.` — twice, on a land that
/// needs untapping and on one that does not.
///
/// The card is old and the assertion is new, and it is about the **journal**
/// rather than about the land. Three things untap a permanent here — the
/// untap step, `Effect::UntapTarget` and now `Effect::UntapSelf` — and until
/// this test the first one journalled `GameEvent::ObjectUntapped` (CR 502.3)
/// and the second wrote the bit in silence. An untap from an effect was
/// therefore absent from the game's own record of what happened: a replay
/// reading the journal would show a land that untapped itself out of
/// nowhere, and a "becomes untapped" trigger — which the DSL has no variant
/// for yet, `Trigger::BecomesTapped` having no twin — would never fire.
///
/// It is written on `UntapTarget` and not on the new `UntapSelf`
/// deliberately. New code cannot have a regression; the rule that was wrong
/// is the one eight cards already use, and a test covering only the newcomer
/// would leave those eight exactly as they were.
///
/// **Both branches, because the quiet one is the easier to get wrong.**
/// Untapping a permanent that is already untapped is not an event, and a
/// door that recorded one anyway would put a "became untapped" in the log
/// for a land that did nothing — which is the same defect as the silence,
/// written the other way round.
#[test]
#[allow(clippy::too_many_lines)] // both branches end to end: a tapped land and an untapped one
fn deserted_temple_says_so_when_it_untaps_a_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(941, forest())
        .battlefield(
            0,
            &[
                deserted_temple(),
                deserted_temple(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = |engine: &Engine<RegistryLookup>, card: CardIndex| -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == card))
            })
            .collect()
    };
    let temples = mine(&engine, deserted_temple());
    let forests = mine(&engine, forest());
    assert_eq!((temples.len(), forests.len()), (2, 3));

    // Two Forests pay the two {1}s; the third stays untapped and is what the
    // quiet branch is aimed at.
    for f in &forests[..2] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: *f })
            .expect("a Forest taps for green");
    }
    assert!(is_tapped(&engine, forests[0]));
    assert!(!is_tapped(&engine, forests[2]));

    let untaps_in = |engine: &Engine<RegistryLookup>, mark: usize, id: ObjectId| -> usize {
        engine.journal().entries()[mark..]
            .iter()
            .filter(|e| {
                matches!(
                    e.event,
                    GameEvent::ObjectUntapped { object, cause }
                        if object == id && cause == Cause::Effect
                )
            })
            .count()
    };
    let aim = |engine: &mut Engine<RegistryLookup>, temple: ObjectId, at: ObjectId| {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: temple,
                    ability_index: 1,
                },
            )
            .expect("a Forest's green pays the {1}");
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("`target land` is a target: {:?}", engine.pending())
        };
        assert!(
            options.contains(&at),
            "any land is a legal target, tapped or not: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![at],
                    players: vec![],
                },
            )
            .expect("a land the engine offered");
        pass_until(engine, stack_is_empty);
    };

    // The land that had something to undo.
    let mark = engine.journal().entries().len();
    aim(&mut engine, temples[0], forests[0]);
    assert!(
        !is_tapped(&engine, forests[0]),
        "the Forest is untapped, which is the half that always worked"
    );
    assert_eq!(
        untaps_in(&engine, mark, forests[0]),
        1,
        "and the journal says it happened, with an effect named as the \
         cause — CR 502.3's turn-based untap has always recorded one, and an \
         untap from an effect recorded nothing at all"
    );

    // The land that did not.
    let mark = engine.journal().entries().len();
    aim(&mut engine, temples[1], forests[2]);
    assert!(!is_tapped(&engine, forests[2]));
    assert_eq!(
        untaps_in(&engine, mark, forests[2]),
        0,
        "an untapped land that is untapped again did not become untapped, \
         and a journal that said otherwise would be inventing an event"
    );
}
