//! Lands, the door `cards/lands/` puts them behind -- and the lands are
//! where most of the engine's mana arithmetic is actually played.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::choice::ChoicePrompt;
use baylee_cards_dsl::counters;
use baylee_cards_dsl::{Effect, Find, SearchDest};

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

    // Four Forests, and two things kept back by name: the Passage, which has
    // to pay its own {T} below, and the Elf, which has to be untapped to
    // attack. This used to read "the Passage is not a basic land, so the
    // CR 305.6 shortcut leaves it untapped" — the helper's defect written
    // down as a feature (#159).
    let passage = on_battlefield(&engine, p0, rogue_s_passage()).expect("the Passage is out");
    tap_mana_where(&mut engine, p0, |id| id != passage && id != elves);
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
                    baylee_cards_dsl::EnterModifier::TappedUnlessCount { filter, .. }
                    | baylee_cards_dsl::EnterModifier::TappedUnlessAtMost { filter, .. } => *filter,
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
        ChoicePrompt::CostSacrifice,
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
            ChoicePrompt::CostTap,
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

fn mystic_gate() -> CardIndex {
    card_index("e9f5feb2-2c1a-46ce-885a-4f378d7d10af")
}

fn fetid_heath() -> CardIndex {
    card_index("42bf259d-4bb9-49c3-b4ec-223dca62f4d6")
}

/// Mystic Gate: `{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.`
///
/// The first **hybrid activation cost** this pool pays. `{W/U}` is one mana
/// of either colour (CR 107.4e), and the whole point of a filter land is that
/// the price is a colour: it turns one coloured mana into two, so a board
/// that can only make colorless cannot start the engine at all. The card was
/// written with `cost!("{1}", …)` — one *generic* mana, payable by anything —
/// and every other reading of it was right, which is why it stood: the
/// `//! Oracle:` header printed `{W/U}`, the effect made its two combination
/// mana, and only the price was wrong. `xtask validate`'s activation-cost
/// check is the half that says so without a game; this is the half that shows
/// what the difference buys an opponent.
///
/// Both halves are asserted, because only the pair is discriminating:
///
/// - **Colorless is refused.** A second Mystic Gate taps for `{C}`, and that
///   `{C}` cannot pay `{W/U}`. Against `{1}` it paid, and the land filtered
///   for free.
/// - **A Plains pays.** One `{W}` in, two picks out — and the two answers may
///   differ, which is `combination: true` and the reason this land is worth
///   playing over a Plains.
#[test]
fn a_filter_land_charges_a_coloured_mana_and_colorless_will_not_do() {
    // All ten of the hybrid cycle — two written by hand, eight by the
    // transcoder — through one rule, which is the point of holding them
    // together: a generated card is a rule's output and a rule is testable.
    // The basic in each row makes the *first* colour of the pair, so every
    // land is paid with a colour its own price names. That White is in both
    // hand-written pairs is the accident that let `{1}` stand: a board that
    // can pay the real price pays the wrong one too, and only a *colorless*
    // board tells the two apart.
    for (land, basic, colors) in [
        (mystic_gate(), plains(), [ManaColor::White, ManaColor::Blue]),
        (
            fetid_heath(),
            plains(),
            [ManaColor::White, ManaColor::Black],
        ),
        (
            cascade_bluffs(),
            island(),
            [ManaColor::Blue, ManaColor::Red],
        ),
        (
            sunken_ruins(),
            island(),
            [ManaColor::Blue, ManaColor::Black],
        ),
        (
            flooded_grove(),
            forest(),
            [ManaColor::Green, ManaColor::Blue],
        ),
        (
            wooded_bastion(),
            forest(),
            [ManaColor::Green, ManaColor::White],
        ),
        (
            fire_lit_thicket(),
            mountain(),
            [ManaColor::Red, ManaColor::Green],
        ),
        (
            rugged_prairie(),
            mountain(),
            [ManaColor::Red, ManaColor::White],
        ),
        (graven_cairns(), swamp(), [ManaColor::Black, ManaColor::Red]),
        (
            twilight_mire(),
            swamp(),
            [ManaColor::Black, ManaColor::Green],
        ),
    ] {
        one_filter_land(land, basic, colors);
    }
}

fn cascade_bluffs() -> CardIndex {
    card_index("f1603384-4361-49c9-98aa-7785fc3504c4")
}

fn sunken_ruins() -> CardIndex {
    card_index("e6415ffb-8b7a-41c3-bedf-0d4112b7b795")
}

fn flooded_grove() -> CardIndex {
    card_index("dc974eb4-72b9-4213-887b-8ee684b93420")
}

fn wooded_bastion() -> CardIndex {
    card_index("61b85077-64aa-4bcc-890d-2d88da9543c0")
}

fn fire_lit_thicket() -> CardIndex {
    card_index("d99a1d9a-7721-4331-bf22-1c6ee0bd825a")
}

fn rugged_prairie() -> CardIndex {
    card_index("8e7641e1-e814-4d5a-9cb3-71ad2f4ceee8")
}

fn graven_cairns() -> CardIndex {
    card_index("5004b84a-33b7-4f6f-b2c2-7086b9087535")
}

fn twilight_mire() -> CardIndex {
    card_index("db623754-e078-4030-ba07-818803c348a8")
}
fn cabal_coffers() -> CardIndex {
    card_index("7358e164-5704-4e78-9b21-6a9bf2a968ce")
}

fn cabal_stronghold() -> CardIndex {
    card_index("066cd584-773c-4623-be53-8f6feda5a26a")
}

fn serra_s_sanctum() -> CardIndex {
    card_index("34187c71-6033-4058-aadc-2bc266f762be")
}

fn tolarian_academy() -> CardIndex {
    card_index("dba4fd31-8931-42dd-bd86-45479c2abf74")
}

fn cloudpost() -> CardIndex {
    card_index("f705c0eb-9c6c-4315-a860-208ed0c5d93e")
}

fn glimmerpost() -> CardIndex {
    card_index("92c9aad6-35ec-425d-be7d-393328992820")
}

/// Five lands that pour "for each …", and the five different questions that
/// phrase turns out to be.
///
/// `Amount::CountOf` has said this since Gaea's Cradle was written by hand;
/// what was missing was a reader for `SVar:X:Count$Valid …`, so eleven of the
/// pool's stubs were refused for an amount the DSL could already spell. Each
/// row is a filter the count would be wrong without, and every one of them is
/// a *different* wrongness:
///
/// - **Cabal Coffers** counts Swamps you control; a Badlands is a Swamp, so
///   the subtype and not the name is the question.
/// - **Cabal Stronghold** prints `basic` in front of the same word, and the
///   same board therefore answers one instead of two. Nothing but that atom
///   separates the two cards.
/// - **Serra's Sanctum** and **Tolarian Academy** count a card *type*, and
///   both say "you control": the opponent's copy is seated deliberately, and
///   a filter that lost `ControlledByYou` would count it.
/// - **Cloudpost** is the one that says the opposite — "each Locus **on the
///   battlefield**" — and it counts itself and the opponent's. This is the
///   case that would be silently narrowed by the condition vocabulary, where
///   a count naming no player is refused outright.
///
/// The colour is measured as a *delta*, because the fixture that pays the
/// price makes mana too.
#[test]
fn a_land_that_counts_pours_one_mana_for_each_thing_its_own_filter_matches() {
    for (seed, land, color, mine, theirs, generic, index, want) in [
        (
            930,
            cabal_coffers(),
            ManaColor::Black,
            vec![swamp(), badlands()],
            vec![swamp()],
            2,
            0,
            2,
        ),
        (
            931,
            cabal_stronghold(),
            ManaColor::Black,
            vec![swamp(), badlands()],
            vec![swamp()],
            3,
            1,
            1,
        ),
        (
            932,
            serra_s_sanctum(),
            ManaColor::White,
            vec![doubling_season()],
            vec![doubling_season()],
            0,
            0,
            1,
        ),
        (
            933,
            tolarian_academy(),
            ManaColor::Blue,
            vec![lightning_greaves(), lightning_greaves()],
            vec![lightning_greaves()],
            0,
            0,
            2,
        ),
        (
            934,
            cloudpost(),
            ManaColor::Colorless,
            vec![glimmerpost()],
            vec![glimmerpost()],
            0,
            0,
            3,
        ),
    ] {
        one_counted_land(seed, land, color, &mine, &theirs, generic, index, want);
    }
}

#[allow(clippy::too_many_arguments)] // a row of the table above, not a call site
fn one_counted_land(
    seed: u64,
    land_card: CardIndex,
    color: ManaColor,
    mine: &[CardIndex],
    theirs: &[CardIndex],
    generic: usize,
    ability_index: u32,
    want: u16,
) {
    let p0 = PlayerId::new(0);
    let mut seated = mine.to_vec();
    seated.push(land_card);
    // Forests pay the price where the card charges one, and a Forest is none
    // of the five things counted here — which is what lets the count be read
    // off one colour.
    seated.extend(std::iter::repeat_n(forest(), generic));
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &seated)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, land_card).expect("the land is on the table");

    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .take(generic)
        .collect();
    for source in forests {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .expect("a Forest taps for {G}");
    }

    let before = engine.state().players[0].mana_pool.available(color);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        )
        .expect("the price is paid and the land is untapped");
    let after = engine.state().players[0].mana_pool.available(color);
    assert_eq!(
        after - before,
        want,
        "the count is the land's own filter, and nothing else on the board"
    );
}

/// Baldur's Gate: `{2}, {T}: Add X mana of any one color, where X is the
/// number of other Gates you control.`
///
/// Two readings meet on one card, and either alone would be wrong:
///
/// - **`Other` is a filter about the card asking**, and `eval::amount` hands
///   `matches` the source object, so the Gate does not count itself. The
///   condition vocabulary refuses `Other` outright for exactly the reason it
///   works here — a condition has no source to be another *than*.
/// - **"any **one** color"** is one pick for the whole amount, which is
///   `mana_choice_dynamic`. `mana_combination` would ask twice and let the
///   two answers differ, which is a different card (the filter cycle above).
#[test]
fn baldurs_gate_counts_the_other_gates_and_asks_once_for_them_all() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(935, forest())
        .battlefield(
            0,
            &[
                baldurs_gate(),
                azorius_guildgate(),
                boros_guildgate(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[azorius_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gate = on_battlefield(&engine, p0, baldurs_gate()).expect("the land is on the table");

    tap_mana_except(&mut engine, p0, gate);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gate,
                ability_index: 1,
            },
        )
        .expect("two Forests pay the {2}");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("one pick for the whole amount: {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any one *color*, so all five are offered");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("a colour the engine offered");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "and no second question, which is what `any one color` means: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two *other* Gates of mine — the Gate itself does not count, and \
         neither does the opponent's"
    );
}

fn baldurs_gate() -> CardIndex {
    card_index("da307ea2-4df7-4d6b-be0f-9dc6ac93db61")
}

fn azorius_guildgate() -> CardIndex {
    card_index("ad1712d8-809f-410c-8b91-ffe6fb8a69a1")
}

fn boros_guildgate() -> CardIndex {
    card_index("73c423b7-cab8-4e69-8070-9edbf96a6c2c")
}

fn one_filter_land(gate_card: CardIndex, basic_card: CardIndex, colors: [ManaColor; 2]) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &[gate_card, gate_card, basic_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gates: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == gate_card))
        })
        .collect();
    let [gate, other] = gates[..] else {
        panic!("two of the land were seated, found {}", gates.len())
    };
    let basic = on_battlefield(&engine, p0, basic_card).expect("the basic is on the table");

    // The other Gate's own first ability, which is the colorless half of the
    // card: `{T}: Add {C}.`
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: other,
                ability_index: 0,
            },
        )
        .expect("a free tap for {C}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );

    // {C} is a mana and it is not a *coloured* one, so it pays no half of
    // `{W/U}` (CR 107.4e, CR 202.2: colorless is not a color).
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gate,
                    ability_index: 1,
                },
            )
            .is_err(),
        "a filter land whose price is generic filters for free; this one \
         charges {{{:?}/{:?}}}",
        colors[0],
        colors[1]
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: basic,
                ability_index: 0,
            },
        )
        .expect("the basic taps for the colour the price names");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gate,
                ability_index: 1,
            },
        )
        .expect("and that colour is one of the two halves of the hybrid");

    for (i, color) in colors.into_iter().enumerate() {
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("pick {i} of two: {:?}", engine.pending())
        };
        assert_eq!(
            options,
            colors.to_vec(),
            "pick {i}: the card's own two colours, in its order"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(color))
            .expect("a colour the engine offered");
    }

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(colors[0]),
            pool.available(colors[1]),
            pool.available(ManaColor::Colorless),
        ),
        (1, 1, 1),
        "one mana went in and two came out, one of each — and the {{C}} \
         that could not pay the price is still floating"
    );
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

// oracle_id = "e43413e4-be17-49af-978a-26210d05f52a"
fn bottomless_vault() -> CardIndex {
    card_index("e43413e4-be17-49af-978a-26210d05f52a")
}

// oracle_id = "4a6625bd-3dd2-45f1-8dc9-034c833fa90c"
fn dwarven_hold() -> CardIndex {
    card_index("4a6625bd-3dd2-45f1-8dc9-034c833fa90c")
}

// oracle_id = "3348df85-e61c-47b5-857d-c79befb38a8a"
fn hollow_trees() -> CardIndex {
    card_index("3348df85-e61c-47b5-857d-c79befb38a8a")
}

// oracle_id = "87a0e0b9-6c2d-47a4-a3ed-7e0ae62fbffc"
fn icatian_store() -> CardIndex {
    card_index("87a0e0b9-6c2d-47a4-a3ed-7e0ae62fbffc")
}

// oracle_id = "48a830f1-8965-4f97-b3d8-ca98eab1ba33"
fn sand_silos() -> CardIndex {
    card_index("48a830f1-8965-4f97-b3d8-ca98eab1ba33")
}

/// Walks until the untap step asks which permanents stay tapped, and hands
/// the question back unanswered.
///
/// `answer_one` answers this one by untapping, which is the right reading
/// for a driver on its way past — and wrong for a test whose subject it is.
/// So the check comes first, before anything is applied.
#[track_caller]
fn walk_to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return (player, options);
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the untap step");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("the untap step never asked");
}

/// Walks to `seat`'s main phase **of the turn it is already in**.
///
/// [`cross_into_the_next_own_main`] waits for the turn number to change,
/// which is one turn too far from inside the untap step of the turn that
/// matters.
#[track_caller]
fn on_to_this_turn_s_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..200 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way to the main phase");
        engine.apply(player, action).expect("the answer is legal");
    }
    panic!("never reached this turn's main phase");
}

/// Bottomless Vault, whole: `This land enters tapped.` / `You may choose
/// not to untap this land during your untap step.` / `At the beginning of
/// your upkeep, if this land is tapped, put a storage counter on it.` /
/// `{T}, Remove any number of storage counters from this land: Add {B} for
/// each storage counter removed this way.`
///
/// The Fallen Empires storage cycle is the pool's one printing of an
/// intervening-`if` clause on a land, and the whole card is built around
/// the player's answer to CR 502.3's determination: a land left tapped
/// banks a counter at the next upkeep, and a land that untaps does not.
/// Both answers are played here in one game, because either on its own
/// proves nothing — a trigger that never fires passes the second half, and
/// a trigger that ignores its clause passes the first.
///
/// The counters are then spent, which is what says the two halves are one
/// card: the number announced at CR 601.2b is the number of {B} that
/// arrives.
#[test]
fn a_storage_land_banks_a_counter_only_on_the_upkeeps_it_spent_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(884, forest())
        .hand(0, &[bottomless_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, bottomless_vault());
    assert!(
        entered_tapped(&engine, land),
        "Bottomless Vault prints `This land enters tapped`"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and arrives empty"
    );

    // First own untap step: the land is the one permanent that prints the
    // sentence, and the answer is to leave it tapped.
    let (player, options) = walk_to_the_untap_question(&mut engine);
    assert_eq!(player, p0, "the active player makes the determination");
    assert_eq!(
        options,
        vec![land],
        "the Forests untap without being asked about"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("leaving it tapped is an answer to the question asked");
    on_to_this_turn_s_main(&mut engine, p0);
    assert!(is_tapped(&engine, land), "it stayed tapped");
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "so the upkeep trigger's clause was true and it banked one"
    );

    // Second own untap step, answered the other way: the land untaps, the
    // clause is false at the beginning of the upkeep, and nothing is banked.
    let (_, options) = walk_to_the_untap_question(&mut engine);
    assert_eq!(options, vec![land], "the same question, a turn later");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("untapping it is the other answer");
    on_to_this_turn_s_main(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "it untapped");
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "and an untapped land banks nothing: the clause is checked, not \
         assumed"
    );

    // And the counter is spendable, one {B} for one counter.
    let (min, max) = spend_storage(&mut engine, p0, land, 2, 1);
    assert_eq!(
        (min, max),
        (0, 1),
        "`any number` is bounded above by what the land actually carries"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one storage counter removed this way is one {{B}}"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and the counter is gone, because removing it was the cost"
    );
}

/// The cycle is one card printed five times, and the only thing that
/// differs is the colour it banks.
///
/// Worth playing all five rather than reading the files: they are generated
/// from the printed text by one rule, so a colour read off the wrong
/// sentence would be identical in every file and invisible to any amount of
/// re-reading. Each land is kept tapped for one upkeep and then spent.
#[test]
fn every_land_of_the_cycle_pays_out_in_its_own_colour() {
    let p0 = PlayerId::new(0);
    for (seed, card, color) in [
        (885, bottomless_vault(), ManaColor::Black),
        (886, dwarven_hold(), ManaColor::Red),
        (887, hollow_trees(), ManaColor::Green),
        (888, icatian_store(), ManaColor::White),
        (889, sand_silos(), ManaColor::Blue),
    ] {
        let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let land = play_land(&mut engine, p0, card);

        let (_, options) = walk_to_the_untap_question(&mut engine);
        assert_eq!(options, vec![land], "{color:?}");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .expect("leaving it tapped");
        on_to_this_turn_s_main(&mut engine, p0);
        assert_eq!(
            counters_on(&engine, land, counters::STORAGE),
            1,
            "{color:?} banked one at its upkeep"
        );

        let (_, options) = walk_to_the_untap_question(&mut engine);
        assert_eq!(options, vec![land], "{color:?}");
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("untapping it");
        on_to_this_turn_s_main(&mut engine, p0);

        spend_storage(&mut engine, p0, land, 2, 1);
        assert_eq!(
            engine.state().players[0].mana_pool.available(color),
            1,
            "one counter, one {color:?}"
        );
    }
}

// oracle_id = "977c2f33-b622-4172-9efb-7f523becd32b"
fn blazemire_verge() -> CardIndex {
    card_index("977c2f33-b622-4172-9efb-7f523becd32b")
}
// oracle_id = "b2eb7a64-a307-4a78-a25d-63fb3ae1e237"
fn cryptic_caves() -> CardIndex {
    card_index("b2eb7a64-a307-4a78-a25d-63fb3ae1e237")
}
// oracle_id = "f1e9abfb-c3c8-483e-b446-5c2afc9f6394"
fn floodfarm_verge() -> CardIndex {
    card_index("f1e9abfb-c3c8-483e-b446-5c2afc9f6394")
}
// oracle_id = "d71bda4c-3dee-4398-8fd0-f77d8743b887"
fn gloomlake_verge() -> CardIndex {
    card_index("d71bda4c-3dee-4398-8fd0-f77d8743b887")
}
// oracle_id = "cce328b9-6100-417e-9ddf-808bbe3e3bc5"
fn hushwood_verge() -> CardIndex {
    card_index("cce328b9-6100-417e-9ddf-808bbe3e3bc5")
}
// oracle_id = "d7e1d4eb-1d4e-460e-9304-7db9ab50ccb5"
fn nimbus_maze() -> CardIndex {
    card_index("d7e1d4eb-1d4e-460e-9304-7db9ab50ccb5")
}
// oracle_id = "2550099d-b3e2-4eb6-9f36-0fc412828ca6"
fn rivendell() -> CardIndex {
    card_index("2550099d-b3e2-4eb6-9f36-0fc412828ca6")
}
// oracle_id = "510a6ac5-f098-4145-ac07-771b1b6f7cdf"
fn riverpyre_verge() -> CardIndex {
    card_index("510a6ac5-f098-4145-ac07-771b1b6f7cdf")
}
// oracle_id = "55a519b4-61cb-448a-875b-4d6dbe00580f"
fn spire_of_industry() -> CardIndex {
    card_index("55a519b4-61cb-448a-875b-4d6dbe00580f")
}
// oracle_id = "a202276b-1f1b-4277-95ee-26877a204f5e"
fn sunbillow_verge() -> CardIndex {
    card_index("a202276b-1f1b-4277-95ee-26877a204f5e")
}
// oracle_id = "439de49b-1091-4688-9ffb-80a025df31c2"
fn tainted_field() -> CardIndex {
    card_index("439de49b-1091-4688-9ffb-80a025df31c2")
}
// oracle_id = "0222414f-98b5-458a-a0fd-831a66cd8b07"
fn tainted_isle() -> CardIndex {
    card_index("0222414f-98b5-458a-a0fd-831a66cd8b07")
}
// oracle_id = "b2bae7fc-0668-4b34-9cd6-0d80aea52275"
fn tainted_peak() -> CardIndex {
    card_index("b2bae7fc-0668-4b34-9cd6-0d80aea52275")
}
// oracle_id = "fa6d05a1-3df4-4751-b1a0-8d9693faec73"
fn tainted_wood() -> CardIndex {
    card_index("fa6d05a1-3df4-4751-b1a0-8d9693faec73")
}
// oracle_id = "cfdd5dc6-593e-495a-8cfe-3a56b3c4c7df"
fn temple_of_the_false_god() -> CardIndex {
    card_index("cfdd5dc6-593e-495a-8cfe-3a56b3c4c7df")
}
// oracle_id = "e861bc08-4f0b-4d22-9b85-9d20227fd5b4"
fn thornspire_verge() -> CardIndex {
    card_index("e861bc08-4f0b-4d22-9b85-9d20227fd5b4")
}
// oracle_id = "c6e0574c-3e2b-4c40-b17a-05bce3d49309"
fn wastewood_verge() -> CardIndex {
    card_index("c6e0574c-3e2b-4c40-b17a-05bce3d49309")
}
// oracle_id = "c52eaa87-9251-4a47-83fd-04e582ade612"
fn willowrush_verge() -> CardIndex {
    card_index("c52eaa87-9251-4a47-83fd-04e582ade612")
}

// oracle_id = "a3fb7228-e76b-4e96-a40e-20b5fed75685"
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Whether printed ability `index` of `card` is on the table right now.
///
/// The mirror of [`activate`], which panics when it is not — and the half
/// this batch of lands actually needs, because "Activate only if …" is a
/// sentence about when the ability is **not** offered.
#[track_caller]
fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

/// The verge cycle and Nimbus Maze: "{T}: Add {X}. Activate only if you
/// control a <land type> or a <land type>."
///
/// Eleven lands, each played twice — once for each land type its clause
/// names — because the clause is an `Or` and a reader that dropped one arm
/// would still pass a test that only ever tried the other. The board starts
/// with the verge alone, which is the half that says the condition is doing
/// anything at all: the unconditional ability beside it is offered in the
/// same breath, so an absent second ability is the clause and not an empty
/// list.
///
/// The land that satisfies the clause is then **played from hand**, so what
/// is asserted is a condition re-read between two priorities rather than
/// one decided when the game was laid out.
///
/// Bleachbone Verge rides along although it is hand-written and older: it
/// is the same sentence written by a person, and this is the one place the
/// two spellings of `Condition::ControlCount` are held against each other
/// in play.
#[test]
fn a_verge_adds_its_second_colour_only_beside_the_land_its_clause_names() {
    let p0 = PlayerId::new(0);
    for (seed, card, key, colour) in [
        (901, blazemire_verge(), swamp(), ManaColor::Red),
        (902, blazemire_verge(), mountain(), ManaColor::Red),
        (903, bleachbone_verge(), plains(), ManaColor::White),
        (904, bleachbone_verge(), swamp(), ManaColor::White),
        (905, floodfarm_verge(), plains(), ManaColor::Blue),
        (906, floodfarm_verge(), island(), ManaColor::Blue),
        (907, gloomlake_verge(), island(), ManaColor::Black),
        (908, gloomlake_verge(), swamp(), ManaColor::Black),
        (909, hushwood_verge(), forest(), ManaColor::White),
        (910, hushwood_verge(), plains(), ManaColor::White),
        (911, riverpyre_verge(), island(), ManaColor::Blue),
        (912, riverpyre_verge(), mountain(), ManaColor::Blue),
        (913, sunbillow_verge(), mountain(), ManaColor::Red),
        (914, sunbillow_verge(), plains(), ManaColor::Red),
        (915, thornspire_verge(), mountain(), ManaColor::Green),
        (916, thornspire_verge(), forest(), ManaColor::Green),
        (917, wastewood_verge(), swamp(), ManaColor::Black),
        (918, wastewood_verge(), forest(), ManaColor::Black),
        (919, willowrush_verge(), forest(), ManaColor::Green),
        (920, willowrush_verge(), island(), ManaColor::Green),
        // Nimbus Maze prints the pair the other way round — the Island
        // makes the {W} and the Plains the {U} — which is exactly the
        // reading a transcoder could get backwards in silence.
        (921, nimbus_maze(), island(), ManaColor::White),
        (922, nimbus_maze(), plains(), ManaColor::Blue),
    ] {
        let index = if card == nimbus_maze() && colour == ManaColor::Blue {
            2
        } else {
            1
        };
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[card])
            .hand(0, &[key])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        assert!(
            offered(&engine, card, 0),
            "seed {seed}: the ability with no clause is offered"
        );
        assert!(
            !offered(&engine, card, index),
            "seed {seed}: and the one with a clause is not, with nothing beside it"
        );

        play_land(&mut engine, p0, key);
        assert!(
            offered(&engine, card, index),
            "seed {seed}: the land it names arrived"
        );
        activate(&mut engine, p0, card, index);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            1,
            "seed {seed}: and it added {colour:?}"
        );
    }
}

/// The four tainted lands: "{T}: Add {B} or {X}. Activate only if you
/// control a Swamp."
///
/// One ability that makes either of two colours, so what is played here is
/// the choice as well as the clause: the options offered are asserted as a
/// pair, and the answer given is the second of them, because a reader that
/// wrote one colour twice would pass a test that took the first.
#[test]
fn a_tainted_land_offers_both_of_its_colours_and_only_beside_a_swamp() {
    let p0 = PlayerId::new(0);
    for (seed, card, colours) in [
        (930, tainted_field(), [ManaColor::White, ManaColor::Black]),
        (931, tainted_isle(), [ManaColor::Blue, ManaColor::Black]),
        (932, tainted_peak(), [ManaColor::Black, ManaColor::Red]),
        (933, tainted_wood(), [ManaColor::Black, ManaColor::Green]),
    ] {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[card])
            .hand(0, &[swamp()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        assert!(
            !offered(&engine, card, 1),
            "seed {seed}: no Swamp, no coloured half"
        );
        play_land(&mut engine, p0, swamp());
        assert!(offered(&engine, card, 1), "seed {seed}: the Swamp arrived");

        activate(&mut engine, p0, card, 1);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "seed {seed}: expected a colour to pick, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            options,
            colours.to_vec(),
            "seed {seed}: both printed colours"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(colours[1]))
            .expect("the colour offered is a legal answer");
        assert_eq!(
            engine.state().players[0].mana_pool.available(colours[1]),
            1,
            "seed {seed}: the colour chosen is the colour added"
        );
    }
}

/// Temple of the False God and Cryptic Caves: "Activate only if you control
/// five or more lands."
///
/// The other shape `Condition::ControlCount` takes — a threshold rather
/// than "at least one" — and the fifth land is played to cross it, so an
/// off-by-one in either direction is visible. The Temple is the pool's one
/// land that prints *nothing* but a conditional ability, which is why it is
/// worth its own row: there is no unconditional half to fall back on, and a
/// clause read as always-false would leave it a land that does nothing at
/// all.
#[test]
fn the_fifth_land_is_what_turns_a_count_of_five_true() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(940, forest())
        .battlefield(
            0,
            &[temple_of_the_false_god(), forest(), forest(), forest()],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        !offered(&engine, temple_of_the_false_god(), 0),
        "four lands is not five"
    );
    play_land(&mut engine, p0, forest());
    assert!(
        offered(&engine, temple_of_the_false_god(), 0),
        "and the fifth is"
    );
    activate(&mut engine, p0, temple_of_the_false_god(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{C}}{{C}}, both of them"
    );

    // Cryptic Caves counts the same way and then spends itself.
    let mut engine = Duel::new(941, forest())
        .battlefield(0, &[cryptic_caves(), forest(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The mana comes first, and that ordering is the test: an ability whose
    // `{1}` nobody can pay is missing from the offer for a reason that has
    // nothing to do with its clause, and asserting on the difference would
    // then prove only that the Forests were untapped. The Caves is kept back
    // because its `{1}, {T}, Sacrifice` is the offer under test and
    // `tap_all_mana` would spend the `{T}` half on mana (#159).
    tap_all_mana_but(&mut engine, p0, Some(cryptic_caves()));
    assert!(
        !offered(&engine, cryptic_caves(), 1),
        "four lands is not five"
    );
    play_land(&mut engine, p0, forest());
    assert!(offered(&engine, cryptic_caves(), 1), "and the fifth is");

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, cryptic_caves(), 1);
    for _ in 0..4 {
        if engine.state().zones.list(ZoneLocation::Hand(p0)).len() > hand {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected while resolving: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "the card it drew"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and the land it sacrificed to draw it"
    );
}

/// Spire of Industry and Rivendell, whose clauses ask about something that
/// is not a land at all: an artifact, and a legendary creature.
///
/// Both are laid out twice rather than played into, because what satisfies
/// them is not a land and could not be put on the table by playing one.
/// Rivendell's ability is not a mana ability either — it goes on the stack
/// like any other — so what the second half asserts is that it was allowed
/// on at all.
#[test]
fn a_clause_may_ask_about_an_artifact_or_a_legend_instead() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(950, forest())
        .battlefield(0, &[spire_of_industry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        !offered(&engine, spire_of_industry(), 1),
        "no artifact, no coloured mana"
    );

    let mut engine = Duel::new(951, forest())
        .battlefield(0, &[spire_of_industry(), lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        offered(&engine, spire_of_industry(), 1),
        "an Equipment is an artifact"
    );
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, spire_of_industry(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected any colour, got {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("any colour includes green");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one mana of the colour chosen"
    );
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "and the life the cost asked for"
    );

    let mut engine = Duel::new(952, forest())
        .battlefield(0, &[rivendell(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(rivendell()));
    assert!(
        !offered(&engine, rivendell(), 1),
        "no legendary creature, no scry"
    );

    let mut engine = Duel::new(953, forest())
        .battlefield(0, &[rivendell(), jin_gitaxias(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Rivendell's own `{T}` is what its second ability costs, so it is kept
    // back from the tap (#159).
    tap_all_mana_but(&mut engine, p0, Some(rivendell()));
    assert!(
        offered(&engine, rivendell(), 1),
        "Jin-Gitaxias is a legendary creature"
    );
    activate(&mut engine, p0, rivendell(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the scry is an ordinary activated ability and uses the stack"
    );
}

// oracle_id = "bf1341dd-41a3-49f6-87ec-63170dde4324"
fn boseiju_who_endures() -> CardIndex {
    card_index("bf1341dd-41a3-49f6-87ec-63170dde4324")
}

/// Boseiju, Who Endures is a Legendary Land that prints two implemented
/// things: "{T}: Add {G}", and "Channel — {1}{G}, Discard this card: Destroy
/// target artifact, enchantment, or nonbasic land an opponent controls."
///
/// One game reads both, because the land half pays for the channel half: the
/// first copy is played as the land drop and tapped for the green in the
/// cost, while the second copy stays in hand and is activated *from there*,
/// so the discard that leaves it in a graveyard is a cost being paid and not
/// a land drop being made.
///
/// The menu the activation offers is the whole grammar of the sentence. The
/// opponent's Sol Ring and their Badlands are on it; the basic Forest beside
/// them is not, which is the one thing "nonbasic land" is worth; the Sol Ring
/// under Boseiju's own controller is on it, because "an opponent controls"
/// sits on the land and not on the artifact; and the nonbasic land this seat
/// already has on the battlefield is *not* on it, because that clause does
/// read the controller. The `Coverage::Partial` gaps — the per-legend cost
/// reduction and the search the victim is granted — are deliberately left
/// unasserted: nothing here may search anybody's library.
#[test]
#[allow(clippy::too_many_lines)] // both printed lines, and the second one's menu
fn boseiju_taps_for_green_and_channels_itself_away_for_an_artifact_or_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest(), quiet_artifact()])
        .hand(0, &[boseiju_who_endures(), boseiju_who_endures()])
        .battlefield(1, &[quiet_artifact(), badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land half, first. A land with no basic land type is no CR 305.6
    // source: "{T}: Add {G}" is a printed ability, offered as (source, 0) in
    // `abilities` and pressed by index rather than by `tap_all_mana`.
    play_land(&mut engine, p0, boseiju_who_endures());
    activate(&mut engine, p0, boseiju_who_endures(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "CR 605.3b: the green is in the pool the moment the land is tapped, \
         with nothing on the stack"
    );

    // The rest of {1}{G}, in the pool *before* the activation is asked for:
    // an activation this board cannot pay for is one the engine never offers.
    tap_all_mana(&mut engine, p0);
    let pool_before = engine.state().players[0].mana_pool.total();
    assert!(
        pool_before >= 2,
        "the channel's {{1}}{{G}} has to be covered by the pool: {pool_before}"
    );

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    let their_land = on_battlefield(&engine, p1, badlands()).expect("their Badlands stands");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their basic Forest stands");
    let my_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my own Sol Ring stands");
    let my_boseiju =
        on_battlefield(&engine, p0, boseiju_who_endures()).expect("the played copy stands");

    // Ability 1 is the channel; ability 0 is the mana ability the copy on the
    // battlefield is already using.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, index)| *index == 1
            && engine
                .state()
                .object(*source)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == boseiju_who_endures()))),
        "the card still in hand offers its channel: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, boseiju_who_endures(), 1);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the channel destroys one target and asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the target");
    assert!(
        options.contains(&rock),
        "\"target artifact\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "\"nonbasic land an opponent controls\": {options:?}"
    );
    assert!(
        options.contains(&my_rock),
        "the controller clause sits on the land, so an artifact of your own \
         is a legal target too: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "and a basic land is not a nonbasic land — the control for the word \
         \"nonbasic\", on a side of the table the controller clause cannot \
         explain away: {options:?}"
    );
    assert!(
        !options.contains(&my_boseiju),
        "while this land *is* nonbasic and is not an opponent's, so the two \
         clauses are read separately: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the two artifacts and the Badlands are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the Sol Ring was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the Sol Ring is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_some(),
        "and only the permanent that was named: the Badlands still stands"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and nothing of this seat's was touched"
    );
    assert!(
        in_graveyard(&engine, p0, boseiju_who_endures()).is_some(),
        "\"Discard this card\" as the cost: the copy activated from hand is in \
         its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, boseiju_who_endures()).is_some(),
        "while the copy played as a land is still on the battlefield — the \
         discard took the card that was activated and not the other printing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 2,
        "{{1}}{{G}} left the pool, so the ability was paid for and not free"
    );
}

fn cephalid_coliseum() -> CardIndex {
    card_index("c733873e-77db-471f-8061-139db24f7e7c")
}

/// Cephalid Coliseum's mana ability is a whole sentence and not the usual
/// "{T}: Add {U}": the same tap also deals 1 damage to the land's own
/// controller. Because that tap is a printed ability rather than a basic
/// land type (CR 305.6), `tap_all_mana` never finds it and the land is
/// pressed by index — one activation then has to do three things at once,
/// put the {U} in the pool with no stack (CR 605.3b), take the life off
/// `p0` alone, and leave the Forest beside it standing, which is what says
/// the mana and the damage both came off the Coliseum and not off the
/// board. The threshold half of the card is the `Coverage::Partial` gap and
/// is deliberately never pressed.
#[test]
fn cephalid_coliseum_taps_for_blue_and_deals_its_own_controller_one_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cephalid_coliseum()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop, so a replacement effect would be seen: the printed
    // card has no enters-tapped clause and the land arrives upright.
    let land = play_land(&mut engine, p0, cephalid_coliseum());
    let bystander = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert!(
        !is_tapped(&engine, land),
        "no enters-tapped clause, so the land drop leaves it upright"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the tap"
    );

    // Ability 0 is "{T}: Add {U}. This land deals 1 damage to you."
    activate(&mut engine, p0, cephalid_coliseum(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "{{T}}: Add {{U}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !is_tapped(&engine, bystander),
        "and the Forest beside it never moved, so the {{U}} has no other \
         source on this board"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the land's controller pays \
         for the mana it makes"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is \"you\": the damage never crosses the table"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

// oracle_id = "c0adbddc-b070-4c5f-afe0-0474c72a9251"
fn gemstone_caverns() -> CardIndex {
    card_index("c0adbddc-b070-4c5f-afe0-0474c72a9251")
}

/// Gemstone Caverns is `Coverage::Partial`: the pre-game luck counter that
/// would turn its tap into "one mana of any color" cannot be expressed, and
/// what is built is the plain `{T}: Add {C}` on a legendary nonbasic land.
/// So the whole supported half is one play — put the land down and tap it —
/// and the pool afterwards holds exactly one *colourless* mana, which is the
/// negative that tells "Add {C}" apart from the unimplemented replacement:
/// nothing was ever asked to name a color.
/// The Caverns is the only permanent on the board (the 60-card backing deck
/// is a library and no land of it was played), so the colourless mana has no
/// other source to have come from, and the printed `{T}` is paid by the land
/// itself rather than through the basic-land-type shortcut of CR 305.6.
#[test]
fn gemstone_caverns_taps_for_one_colourless_and_names_no_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, forest())
        .hand(0, &[gemstone_caverns()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, gemstone_caverns()).expect("the Caverns are in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on an empty board");
    let caverns = on_battlefield(&engine, p0, gemstone_caverns()).expect("the land hit the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land nobody tapped is no mana"
    );

    // `{T}: Add {C}` is a printed ability on a nonbasic land, so it is an
    // ordinary entry in `abilities` and not the intrinsic CR 305.6 list.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "playing a land leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(caverns, 0)),
        "the land's own {{T}} is offered like any other printed ability: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, gemstone_caverns(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "`Add {{C}}` is the half of the card that is written"
    );
    assert_eq!(
        pool.total(),
        1,
        "and nothing came with it: only the one mana ability resolved"
    );
    assert!(
        is_tapped(&engine, caverns),
        "the {{T}} was paid by the Caverns"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "so the seat is back where it was and was never asked for a color — \
         the luck-counter replacement is not in play: {:?}",
        engine.pending()
    );
}

// oracle_id = "91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4"
fn inventors_fair() -> CardIndex {
    card_index("91d4a5fe-fd6d-4b14-a63f-61b4d0ecd9c4")
}

/// Inventors' Fair — a legendary land printing "{T}: Add {C}", an upkeep
/// trigger that gains 1 life whenever you control three or more artifacts,
/// and "{4}, {T}, Sacrifice this land: search your library for an artifact
/// card … Activate only if you control three or more artifacts."
///
/// Both artifact-gated halves are read off two boards that differ only in the
/// count: two artifacts against three. The smaller board reaches its own main
/// phase on twenty life and floats the tutor's whole {4} across two tapped
/// Sol Rings without being offered it, so the missing third artifact is the
/// only thing that can be withholding either line; the larger board is one
/// life up on the same walk, is offered the tutor, and pays for it with the
/// land — which is in its owner's graveyard afterwards. The land's own {T} is
/// pressed on the smaller board too, so all three printed lines are played and
/// neither refusal can be blamed on a tapped land or a missing mana ability.
#[test]
#[allow(clippy::too_many_lines)] // two boards, because the gate has two answers
fn inventors_fair_gates_its_lifegain_and_its_tutor_on_three_artifacts() {
    let p0 = PlayerId::new(0);

    // Two artifacts: the printed count is one short, and the tutor's {4} is
    // floating before the offer is read.
    let mut poor = Duel::new(31, quiet_artifact())
        .battlefield(0, &[inventors_fair(), quiet_artifact(), quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut poor);
    assert!(walk_to_own_main(&mut poor, p0), "p0 reaches its own main");
    assert_eq!(
        poor.state().players[0].life,
        20,
        "two artifacts is not three, so the upkeep trigger gained nothing",
    );

    let held = on_battlefield(&poor, p0, inventors_fair()).expect("the Fair is on the table");
    for _ in 0..2 {
        activate(&mut poor, p0, quiet_artifact(), 0);
    }
    assert_eq!(
        poor.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4,
        "two Sol Rings pay {{C}}{{C}} apiece",
    );
    let Pending::Priority { legal, .. } = poor.pending().clone() else {
        panic!("expected priority, got {:?}", poor.pending())
    };
    assert!(
        !legal.abilities.contains(&(held, 2)),
        "the tutor's whole {{4}} is in the pool and the land is untapped, so \
         only the two-artifact board withholds it: {:?}",
        legal.abilities,
    );

    // Ability 1 is the printed "{T}: Add {C}".
    activate(&mut poor, p0, inventors_fair(), 1);
    assert_eq!(
        poor.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        5,
        "the Fair's own tap added the single colorless it prints",
    );
    assert!(is_tapped(&poor, held), "which tapped the land");
    assert!(
        stack_is_empty(&poor),
        "CR 605.3b: a mana ability never uses the stack",
    );

    // Three artifacts: the same two readings, both the other way round.
    let mut rich = Duel::new(31, quiet_artifact())
        .battlefield(
            0,
            &[
                inventors_fair(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut rich);
    assert!(walk_to_own_main(&mut rich, p0), "p0 reaches its own main");
    assert_eq!(
        rich.state().players[0].life,
        21,
        "three artifacts at the beginning of your upkeep is one life",
    );

    let fair = on_battlefield(&rich, p0, inventors_fair()).expect("the Fair is on the table");
    for _ in 0..3 {
        activate(&mut rich, p0, quiet_artifact(), 0);
    }
    let Pending::Priority { legal, .. } = rich.pending().clone() else {
        panic!("expected priority, got {:?}", rich.pending())
    };
    assert_eq!(
        rich.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        6,
        "three Sol Rings, which is the tutor's {{4}} and then some",
    );
    assert!(
        legal.abilities.contains(&(fair, 2)),
        "the same count that fired the trigger now opens the tutor: {:?}",
        legal.abilities,
    );

    let library_before = library_size(&rich, p0);
    let hand_before = rich.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 2 is the tutor, whose cost is {4}, {T} and the sacrifice. It is
    // an ordinary activated ability, so the costs are paid now and the search
    // is a question the *resolution* asks: the stack is in between.
    activate(&mut rich, p0, inventors_fair(), 2);
    pass_until(&mut rich, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = rich.pending().clone()
    else {
        panic!("the search asks which artifact, got {:?}", rich.pending())
    };
    assert_eq!(player, p0, "the seat that paid is the seat that searches");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a search of the library, not a cost being paid",
    );
    assert!(
        !options.is_empty(),
        "the deck this duel was dealt is made of artifacts",
    );
    let found = options[0];
    rich.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .expect("a card the search offered");
    pass_until(&mut rich, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&rich, p0),
        library_before - 1,
        "\"search your library for an artifact card\" — one card left it",
    );
    assert_eq!(
        rich.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and arrived in hand",
    );
    assert!(
        on_battlefield(&rich, p0, inventors_fair()).is_none(),
        "the sacrifice is paid from the battlefield",
    );
    assert!(
        in_graveyard(&rich, p0, inventors_fair()).is_some(),
        "so the land is in its owner's graveyard",
    );
}

// oracle_id = "e9b6a394-691c-425a-9307-76d8edc7375e"
fn otawara_soaring_city() -> CardIndex {
    card_index("e9b6a394-691c-425a-9307-76d8edc7375e")
}

/// Otawara, Soaring City prints a plain `{T}: Add {U}` and, beside it, a
/// Channel line that is not a land ability at all: "{3}{U}, Discard this
/// card: Return target artifact, creature, enchantment, or planeswalker to
/// its owner's hand", activated from hand. This plays the Channel half, so
/// both readings that make it what it is are struck: the card leaves the
/// *hand* as a cost — the graveyard and not the battlefield has to hold it
/// afterwards — and the creature across the table is bounced to its owner's
/// hand while the Forest beside it, a land and so none of the four named
/// card types, is never offered as a target in the first place.
#[test]
fn otawara_channels_from_hand_to_bounce_a_creature_and_discards_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(97, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[otawara_soaring_city()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );
    assert!(
        in_hand(&engine, p0, otawara_soaring_city()).is_some(),
        "the Channel line is activated from hand, which is where the card is"
    );

    // The cost is {3}{U} and the engine reads it off the *pool*, so the mana
    // goes in first: four Islands are the basic land type of CR 305.6, which
    // is the whole of what `tap_all_mana` taps.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        4,
        "four Islands pay three generic and the one blue"
    );

    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let library_after_channel = library_size(&engine, p0);

    // Ability 1 is the Channel line. Ability 0 is the printed mana ability,
    // which a card in hand cannot pay a {T} for and so is never offered.
    activate(&mut engine, p0, otawara_soaring_city(), 1);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Channel line asks for its target before anything is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the target");
    assert_eq!((min, max), (1, 1), "exactly one permanent comes back");
    assert!(
        options.contains(&their_elves),
        "\"target artifact, creature, …\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "a land is none of the four card types the sentence names: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves],
            },
        )
        .expect("the creature the question offered is the one it was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, otawara_soaring_city()).is_some(),
        "discarding this card is the cost, so it never reached the battlefield \
         and lies in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the Elf left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "…to its *owner's* hand, the seat that cast nothing"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the permanent that was never a legal target never moved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_after_channel,
        "a bounce draws nothing, for either seat"
    );
}

/// Walks to the surveil question and reads what it offers.
///
/// Four tests ask it, which is why it is a helper: "When this land enters,
/// surveil 1" is an ordinary trigger, so it uses the stack and every one of
/// them has to pass priority to it first — and a walk written four times is
/// a walk that drifts.
#[track_caller]
fn surveil_offer(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> (Vec<ObjectId>, u8, u8) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SurveilGraveyard,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the walk above stops on nothing else");
    };
    assert_eq!(player, seat, "the seat that surveils is the seat asked");
    (options, min, max)
}

fn raucous_theater() -> CardIndex {
    card_index("04e5e84f-8fd4-43ab-8f9d-5b24646f7ae5")
}

/// Raucous Theater prints `Land — Swamp Mountain`, "This land enters tapped",
/// "{T}: Add {B} or {R}" and "When this land enters, surveil 1" — and all
/// three are read here. The land is *played*, not seated: a
/// `starting_battlefield` permanent is a placement with no entry for a
/// replacement effect to look at, so a board built that way arrives untapped
/// whatever the card says. The following untap step is the other half — this
/// is a tapped entry, not a permanent held down (CR 502.3).
///
/// This is the *binning* direction of CR 701.25a: the one card the surveil
/// looked at is put into the graveyard, and both zones are counted, because
/// "look at the top card" and "put it in the graveyard" are different halves
/// and a surveil that quietly did neither would pass a test that only counted
/// the library. Thundering Falls below is the keeping direction.
#[test]
fn raucous_theater_enters_tapped_and_taps_for_black_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[raucous_theater()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .copied()
        .expect("p0 has a library");

    let land = play_land(&mut engine, p0, raucous_theater());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\", read off a real land drop"
    );

    // "When this land enters, surveil 1."
    let (options, min, max) = surveil_offer(&mut engine, p0);
    assert_eq!(
        options,
        vec![top_before],
        "surveil 1 looks at exactly the top card of its own library"
    );
    assert_eq!(
        (min, max),
        (0, 1),
        "any number of them, which here is 0 or 1"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![top_before],
            },
        )
        .expect("the card it just looked at");

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the card it binned left the library"
    );
    assert_eq!(
        in_graveyard(&engine, p0, forest()),
        Some(top_before),
        "and it is in the graveyard, which is where a surveil differs from a \
         scry"
    );

    // One turn cycle: an entry that taps is not a permanent that never untaps.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step gives the land back, so it entered tapped and was \
         not held down"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}".
    activate(&mut engine, p0, raucous_theater(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours and nothing else: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "{{B}} and {{R}} are the whole of the sentence: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, off the land's own {{T}}"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing beside it");
    assert!(is_tapped(&engine, land), "which tapped the land");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}

// oracle_id = "ac2dd694-d2f1-4025-8400-12332bdc882a"
fn takenuma_abandoned_mire() -> CardIndex {
    card_index("ac2dd694-d2f1-4025-8400-12332bdc882a")
}

/// Takenuma, Abandoned Mire is a legendary land with no enter modifier, and
/// the half of it that is written is `{T}: Add {B}`. The card prints no basic
/// land type, so this tap is a *printed* ability and `tap_all_mana` — the
/// CR 305.6 intrinsic list — never touches it, which is why it is pressed by
/// index instead. A Forest stands untapped beside it as the control: black
/// mana in the pool while the only other land on the board has not moved can
/// have come from nowhere else. The channel line is the `Coverage::Partial`
/// gap, so it is asserted as an absence — nothing discards the card, and it
/// is still on the battlefield in nobody's graveyard afterwards.
#[test]
fn takenuma_abandoned_mire_taps_for_black_and_offers_no_channel() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[takenuma_abandoned_mire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let land = play_land(&mut engine, p0, takenuma_abandoned_mire());
    assert!(
        !entered_tapped(&engine, land),
        "the card prints no enter modifier, so it lands untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land makes no mana by itself"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the land's own {{T}}: Add {{B}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&land),
        "a printed tap is no basic land type (CR 305.6), so `tap_all_mana` \
         would never press it: {:?}",
        legal.mana_abilities
    );

    activate(&mut engine, p0, takenuma_abandoned_mire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}}, off the land's own tap"
    );
    assert_eq!(pool.total(), 1, "one mana and nothing else came with it");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !is_tapped(&engine, forest_land),
        "and the untapped Forest is the control: the black mana has no other \
         source on this board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        on_battlefield(&engine, p0, takenuma_abandoned_mire()).is_some(),
        "the channel line is the gap, so nothing discarded the card for it"
    );
    assert!(
        in_graveyard(&engine, p0, takenuma_abandoned_mire()).is_none(),
        "and no three cards were milled into anybody's graveyard"
    );
}

// oracle_id = "d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e"
fn thundering_falls() -> CardIndex {
    card_index("d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e")
}

/// Thundering Falls — Land — Island Mountain: "This land enters tapped.
/// {T}: Add {U} or {R}. When this land enters, surveil 1."
///
/// The land is *played* rather than seeded, because a permanent placed with
/// `starting_battlefield` is a `Cause::Setup` placement that no entry
/// replacement effect ever looks at — so only a real land drop can show the
/// printed tapped entry. The next turn then reads two things at once — the
/// untap step stands the land back up, so the tapped status belonged to the
/// entry and not to the card, and the printed `{T}` ability offers exactly
/// blue and red, which is the choice the card has to print for itself because
/// two basic land types cannot express "or".
///
/// The surveil is the **keeping** direction here, which is the half that is
/// easy to get wrong and impossible to see: answering "none of them" has to
/// leave the card exactly where it was. A surveil that binned on an empty
/// answer, or that dropped the card out of the library into nothing, would
/// look identical from the battlefield. Raucous Theater above bins.
#[test]
fn thundering_falls_enters_tapped_then_taps_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[thundering_falls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .copied()
        .expect("p0 has a library");
    let falls = play_land(&mut engine, p0, thundering_falls());
    assert!(
        entered_tapped(&engine, falls),
        "\"This land enters tapped\" — and it was played, so the entry \
         modifier is the only thing that could have tapped it"
    );

    let _ = surveil_offer(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("keeping everything is an answer (CR 701.25a: \"any number\")");
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing was put into a graveyard, so nothing left the library"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(top_before),
        "and the card that was looked at is still the top one"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "no Forest of the filler library reached a graveyard"
    );

    // A whole turn cycle, so CR 502.3's untap step is what stands the land
    // back up — the reading above is the entry, not a permanent condition.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !entered_tapped(&engine, falls),
        "no effect holds this land down, so the untap step untapped it"
    );

    // Ability 0 is the printed "{T}: Add {U} or {R}."
    activate(&mut engine, p0, thundering_falls(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "two colours and no third thing: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "\"or {{R}}\" is on the menu with the {{U}} an Island would make: \
         {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "the other half of the choice was not handed over as well"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, falls), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}

// oracle_id = "08d80efc-9542-4ba2-824c-c8615d8d07f2"
fn undercity_sewers() -> CardIndex {
    card_index("08d80efc-9542-4ba2-824c-c8615d8d07f2")
}

/// Undercity Sewers is a Land — Island Swamp whose printed sentences are
/// "This land enters tapped", "{T}: Add {U} or {B}" and an enters-surveil.
/// The surveil is the subject of the two tests above and is merely walked
/// past here — `pass_until` keeps everything, which is why this test may
/// count a library at all. The land is *played* rather than
/// seeded onto the battlefield, which is the only way the tapped entry is a
/// rule at all: `starting_battlefield` places a permanent with
/// `Cause::Setup`, and no replacement effect looks at a placement. The turn
/// cycle that follows keeps the second half honest — the same permanent is
/// untapped again afterwards, so the {T} this test pays is a real cost on a
/// real land and not a free tap on a frozen one.
#[test]
fn undercity_sewers_enters_tapped_and_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // The entry, with the control beside it: the same helper, the same seat,
    // the same first main phase — a Forest, which arrives untapped. Without
    // that, the assertion below would hold for a harness that taps whatever
    // it plays.
    let (mut engine, land) = play_land_face(undercity_sewers(), 0)
        .expect("a land in hand, on an empty board, in a first main phase");
    let (control_engine, forest) =
        play_land_face(basic_forest(), 0).expect("and the same for a basic Forest");
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\""
    );
    assert!(
        !entered_tapped(&control_engine, forest),
        "the harness' own land drop taps nothing by itself"
    );

    // Through the opponent's turn and back: `walk_to_own_main` answers "you
    // are already there" from the main phase this started in, so the walk has
    // to leave it first.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn and reaches its own main phase"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step gives the Sewers back, so the sentence above was the \
         entry and not a permanent that never untaps"
    );

    // {T}: Add {U} or {B}. Both basic land types sit on one face, so the card
    // prints the ability itself and it is pressed like any other — and it
    // asks which of the two colours this tap is for.
    activate(&mut engine, p0, undercity_sewers(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours it prints and no third: {options:?}"
    );
    assert!(options.contains(&ManaColor::Blue), "{options:?}");
    assert!(options.contains(&ManaColor::Black), "{options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the other half of the choice was not paid"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}

fn idyllic_grange() -> CardIndex {
    card_index("23d349a0-e441-40b8-b634-13e61440a7c8")
}

fn dwarven_mine() -> CardIndex {
    card_index("74ed0bd3-ac31-41a4-8220-d8e7c8c1c437")
}

fn gingerbread_cabin() -> CardIndex {
    card_index("fa98c367-0312-49c6-abef-72e5ead4cc7d")
}

/// The Eldraine trio: "This land enters tapped unless you control three or
/// more other \[basics of its own type\]. When this land enters untapped, …".
///
/// Two sentences and they are wired to each other, which is what this plays.
/// The count is `at_least: 3` and the "other" is the engine's, so a Grange
/// standing beside two Plains is the third Plains and still not three
/// *other* ones. And the second sentence is gated on the first: a land that
/// came down tapped triggers nothing at all, which is why the token half is
/// asserted on both sides rather than only where it appears.
///
/// Dwarven Mine and Gingerbread Cabin carry the half that is observable
/// without a target — a Dwarf and a Food — while Idyllic Grange's counter
/// wants a creature to go on, so it is played here for the tapped half.
#[test]
fn an_eldraine_land_wants_three_others_of_its_own_type() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(214, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[dwarven_mine(), idyllic_grange()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = play_land(&mut engine, p0, dwarven_mine());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, mine),
        "two Mountains are not three other Mountains"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "a land that entered tapped makes no Dwarf"
    );
}

/// The other side of the same sentence, one Mountain further along.
#[test]
fn an_eldraine_land_that_enters_untapped_makes_its_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(215, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[dwarven_mine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = play_land(&mut engine, p0, dwarven_mine());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !entered_tapped(&engine, mine),
        "three Mountains are three other Mountains"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "the Mine makes one Dwarf on the way in"
    );
}

/// Gingerbread Cabin is the same rule over Forests, and it is here because
/// the three cards read the *subtype* out of their own filter: a rule that
/// wrote the wrong one would still pass every assertion above.
#[test]
fn a_gingerbread_cabin_counts_forests_and_not_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(216, forest())
        .battlefield(0, &[forest(), forest(), island()])
        .hand(0, &[gingerbread_cabin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cabin = play_land(&mut engine, p0, gingerbread_cabin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, cabin),
        "two Forests and an Island are not three other Forests"
    );
    assert!(tokens_of(&engine, p0).is_empty(), "no Food either");
}

/// The five Turbulent lands: "This land enters tapped unless your
/// **opponents** control eight or more lands."
///
/// The first count in this pool that looks across the table.
/// `controls_at_least` walks the whole battlefield and asks
/// `eval::matches` with the land's own controller as "you", so
/// `Filter::ControlledByOpponent` is answered from the *land's* side — and
/// the seven lands seat 0 is standing on are the bystander that says so. Get
/// the side wrong and every one of these five enters untapped on turn one
/// off your own mana base.
///
/// Eight is also the largest `at_least` the pool has, four past the battle
/// lands' two, so the comparison is exercised somewhere it cannot be
/// confused with a presence test.
///
/// All five are played because each names its own `CardIndex`, and a cycle
/// is exactly where one file quietly gets a neighbour's number.
#[test]
fn a_turbulent_land_counts_the_lands_across_the_table() {
    const TURBULENT: &[(&str, &str, [ManaColor; 2])] = &[
        (
            "114dd40d-5ad8-4913-a08f-572b9521eb5b",
            "Turbulent Fen",
            [ManaColor::Black, ManaColor::Green],
        ),
        (
            "2eb4da30-2600-4a7f-8e6c-6a090faa9a8d",
            "Turbulent Moor",
            [ManaColor::White, ManaColor::Black],
        ),
        (
            "9aef7510-9f06-4939-8cae-f71330d1105e",
            "Turbulent Springs",
            [ManaColor::Blue, ManaColor::Red],
        ),
        (
            "db444f9d-4dde-4308-b0f2-7acfe6de871a",
            "Turbulent Steppe",
            [ManaColor::Red, ManaColor::White],
        ),
        (
            "bd8adca6-4f16-45f8-994a-fe55bd573bd0",
            "Turbulent Wilderness",
            [ManaColor::Green, ManaColor::Blue],
        ),
    ];
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, colors)) in TURBULENT.iter().enumerate() {
        let seed = u64::try_from(seed).expect("five rows");
        let land = card_index(oracle);
        // Seven across the table and seven of your own: one short on the
        // side that counts, and seven too many on the side that does not.
        let seven = [forest(); 7];
        let mut engine = Duel::new(300 + seed, forest())
            .battlefield(0, &seven)
            .battlefield(1, &seven)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: seven opponent lands are not eight"
        );

        let eight = [forest(); 8];
        let mut engine = Duel::new(400 + seed, forest())
            .battlefield(0, &[])
            .battlefield(1, &eight)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);
        pass_until(&mut engine, stack_is_empty);
        assert!(
            !entered_tapped(&engine, played),
            "{name}: eight opponent lands, and none of your own needed"
        );

        // And it taps. Two basic land types mean no CR 305.6 shortcut — the
        // engine refuses to pick a colour on the player's behalf — so the
        // whole of this land's mana is the `AddManaChoice` its card prints,
        // and the first five of these were written without one: `Land —
        // Swamp Forest`, `Coverage::Implemented`, and untappable.
        // Once per colour rather than once per land: a mana ability that
        // answers the first colour and never the second is exactly the
        // Godless Shrine bug the CR 305.6 shortcut refuses to repeat.
        for (i, colour) in colors.iter().enumerate() {
            let mut engine = Duel::new(
                500 + seed * 2 + u64::try_from(i).expect("two colours"),
                forest(),
            )
            .battlefield(1, &eight)
            .hand(0, &[land])
            .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);
            let played = play_land(&mut engine, p0, land);
            pass_until(&mut engine, stack_is_empty);
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: played,
                        ability_index: 0,
                    },
                )
                .unwrap_or_else(|err| panic!("{name}: refused its own mana ability: {err:?}"));
            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!("{name}: made mana without asking which colour");
            };
            assert_eq!(
                options.as_slice(),
                colors.as_slice(),
                "{name}: offered the wrong colours"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(*colour))
                .unwrap_or_else(|err| panic!("{name}: refused {colour:?}: {err:?}"));
            let pool = &engine.state().players[0].mana_pool;
            assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
            assert_eq!(
                pool.available(*colour),
                1,
                "{name}: and it is the colour that was chosen"
            );
        }
    }
}

fn uncharted_haven() -> CardIndex {
    card_index("d23c3613-bc5e-4fc5-939c-62a090c53a79")
}

/// Every land in the pool printing "As it enters, choose a color" with no
/// exception, and the colour each one is told to make here.
///
/// One table rather than five tests, because they are one card with five
/// names: the enchantment and the Desert and the snow land differ in their
/// type line and in nothing this is about. The colours are spread across the
/// rows so all five are named at least once.
const ANY_COLOUR_LANDS: [(&str, &str, ManaColor); 5] = [
    (
        "d23c3613-bc5e-4fc5-939c-62a090c53a79",
        "Uncharted Haven",
        ManaColor::White,
    ),
    (
        "e103f422-85c0-43f8-8a2f-8b7863e503fa",
        "Mirage Mesa",
        ManaColor::Blue,
    ),
    (
        "660d44a2-391a-416c-b46c-ddcc3739f527",
        "Valgavoth's Lair",
        ManaColor::Black,
    ),
    (
        "2ac34f3e-822d-4fde-99ca-a31c4d9503fd",
        "Shimmerdrift Vale",
        ManaColor::Red,
    ),
    (
        "b26cfeb0-7bbe-4d93-8eed-e832f175a80c",
        "Crossroads Village",
        ManaColor::Green,
    ),
];

/// "This land enters tapped. As it enters, choose a color. {T}: Add one mana
/// of the chosen color."
///
/// Two halves that only work as a pair: `EnterModifier::ChooseColor` writes
/// the colour onto the permanent and `ManaSource::Chosen` reads it back off
/// the same object. The card says nothing about which colour, which is the
/// point — two of these on one battlefield are two different lands.
///
/// The tapping is asserted here and not somewhere cheaper because this is the
/// card that found the bug: an entry modifier that *asks* returns from the
/// scan, and everything the card printed behind it used to be dropped. A
/// Haven that only asked a question entered untapped.
#[test]
fn a_land_that_chooses_any_colour_taps_for_the_one_its_controller_named() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, colour)) in ANY_COLOUR_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 700 + u64::try_from(seed).expect("five rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);

        // The choice is asked as the land enters, before anything else can
        // happen, and colorless is not a colour (CR 105.1).
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "{name}: no colour was asked for, pending is {:?}",
                engine.pending()
            );
        };
        assert_eq!(
            options,
            baylee_cards_dsl::ALL_MANA_COLORS.to_vec(),
            "{name}: every colour and nothing else"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*colour))
            .expect("its own colour list");

        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: it enters tapped as well"
        );

        // Untap it the honest way: round to this seat's next main phase.
        // Then it taps for exactly the colour that was named, with nothing
        // left to choose.
        let from = engine.state().turn.number;
        pass_until(&mut engine, |e| {
            e.state().turn.number > from
                && e.state().turn.active == p0
                && matches!(e.state().turn.phase, Phase::FirstMain)
        });
        assert!(
            !is_tapped(&engine, played),
            "{name}: a turn cycle untapped it"
        );
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: played,
                    ability_index: 0,
                },
            )
            .expect("the land taps for the chosen colour");
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
        assert_eq!(
            pool.available(*colour),
            1,
            "{name}: and it is the colour named"
        );
    }
}

/// The other ten: every land printing "choose a color other than <c>" beside
/// "{T}: Add {c} or one mana of the chosen color".
///
/// Two cycles wearing one rule — five Gates and five Thriving lands — so the
/// row carries the colour the card prints (which is the one it may not be
/// told to make) and a second colour to name, walked around WUBRG so no two
/// rows ask the same pair.
const EXCLUDING_LANDS: [(&str, &str, ManaColor, ManaColor); 10] = [
    (
        "15f1fe23-5af4-4fc4-8cde-2e0bf9f9be0c",
        "Citadel Gate",
        ManaColor::White,
        ManaColor::Blue,
    ),
    (
        "b574c540-9f8a-4fd4-8809-d02c9b099ddc",
        "Sea Gate",
        ManaColor::Blue,
        ManaColor::Black,
    ),
    (
        "dde6bce5-8bbe-4866-b5aa-2c05c7d37241",
        "Black Dragon Gate",
        ManaColor::Black,
        ManaColor::Red,
    ),
    (
        "1999b5ac-21fb-4d99-ad72-58bf507f9a59",
        "Cliffgate",
        ManaColor::Red,
        ManaColor::Green,
    ),
    (
        "dd6e67c0-66a1-49b7-8a86-3cf4b209fd07",
        "Manor Gate",
        ManaColor::Green,
        ManaColor::White,
    ),
    (
        "d1946630-e224-40db-8f0d-388b09622288",
        "Thriving Heath",
        ManaColor::White,
        ManaColor::Black,
    ),
    (
        "69fc70b8-b143-4662-ac95-e2743037239d",
        "Thriving Isle",
        ManaColor::Blue,
        ManaColor::Red,
    ),
    (
        "bff416bb-d193-4c45-b2c1-7c297dbfad08",
        "Thriving Moor",
        ManaColor::Black,
        ManaColor::Green,
    ),
    (
        "91fceb34-0f2d-4392-be27-00dcd765637f",
        "Thriving Bluff",
        ManaColor::Red,
        ManaColor::White,
    ),
    (
        "a8052556-8962-4130-86a8-6fb7b6a324f7",
        "Thriving Grove",
        ManaColor::Green,
        ManaColor::Blue,
    ),
];

/// "As it enters, choose a color other than white. {T}: Add {W} or one mana
/// of the chosen color."
///
/// Two halves again, and both of them narrower than the Haven's:
/// `ChooseColorExcept` takes the printed colour off the list the player is
/// offered, and `ManaSource::ChosenOr` puts it back on the one the *ability*
/// offers — the colour the land cannot be told to make is the colour it
/// always makes.
#[test]
fn a_land_that_excludes_its_own_colour_still_taps_for_it() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, printed, named)) in EXCLUDING_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 720 + u64::try_from(seed).expect("ten rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);

        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "{name}: no colour was asked for, pending is {:?}",
                engine.pending()
            );
        };
        let offered: Vec<ManaColor> = baylee_cards_dsl::ALL_MANA_COLORS
            .iter()
            .copied()
            .filter(|c| c != printed)
            .collect();
        assert_eq!(
            options, offered,
            "{name}: its printed colour is the one it may not be told to make"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*named))
            .expect("a colour off its own list");
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: it enters tapped as well"
        );

        let from = engine.state().turn.number;
        pass_until(&mut engine, |e| {
            e.state().turn.number > from
                && e.state().turn.active == p0
                && matches!(e.state().turn.phase, Phase::FirstMain)
        });
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: played,
                    ability_index: 0,
                },
            )
            .expect("the land taps");
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("{name}: a two-option mana ability asked nothing");
        };
        assert_eq!(
            options,
            vec![*printed, *named],
            "{name}: its printed colour and the one that was named, and no others"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*printed))
            .expect("the colour the card prints");
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
        assert_eq!(
            pool.available(*printed),
            1,
            "{name}: the printed colour, which the choice had excluded"
        );
    }
}

fn reflecting_pool() -> CardIndex {
    card_index("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
}

/// A Reflecting Pool beside an Uncharted Haven sees the colour the Haven was
/// told to make.
///
/// The Pool reads `produced_colors`, which is a reading of the *card* — and a
/// land whose whole mana is "one mana of the chosen color" has nothing there
/// to read, because the card cannot know. So the Pool was looking at a land
/// that makes nothing. It takes both halves: `produced_chosen` is the card's
/// ("this one reads a chosen colour") and `GameObject::chosen_color` is the
/// object's, because the field is written by an entry and says nothing on its
/// own about what the permanent does with it.
#[test]
fn a_reflecting_pool_sees_the_colour_a_neighbour_was_told_to_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(731, forest())
        .battlefield(0, &[reflecting_pool()])
        .hand(0, &[uncharted_haven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pool = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == reflecting_pool())
        })
        .expect("the Pool was seated");

    play_land(&mut engine, p0, uncharted_haven());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black is a colour");
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pool,
                ability_index: 0,
            },
        )
        .expect("the Pool taps");
    // One option is not a choice, so nothing is asked and the mana is simply
    // added (`resolve::mana` short-circuits a single colour).
    let mana = &engine.state().players[0].mana_pool;
    assert_eq!(
        mana.available(ManaColor::Black),
        1,
        "the colour its neighbour was told to make"
    );
    assert_eq!(
        mana.total(),
        1,
        "and nothing else: the Haven makes one thing"
    );
}

/// Every land in the pool whose *entry* surveils, minus the three with tests
/// of their own above.
///
/// The Murders at Karlov Manor duals and the three Deserts that follow them
/// print one sentence between them — "When this land enters, surveil 1" —
/// and a table is what says so. The `bin` column walks both answers down the
/// list, because "put it in the graveyard" and "leave it on top" are the two
/// halves of CR 701.25a and a surveil that ignored the answer would satisfy
/// either one alone.
const ENTRY_SURVEIL_LANDS: [(&str, &str, bool); 10] = [
    (
        "b33656ae-3473-4223-845f-f9147f87678b",
        "Commercial District",
        true,
    ),
    (
        "9ea747cf-5d04-4aa7-bdc3-8145860cd1ba",
        "Elegant Parlor",
        false,
    ),
    ("ca4b6689-04ee-4227-9bdc-cb5a9590c745", "Hedge Maze", true),
    (
        "d51831b1-7394-456e-a1de-6787a59f5932",
        "Lush Portico",
        false,
    ),
    (
        "ccfb8b4d-651c-418a-aa19-cb23105b3f2f",
        "Meticulous Archive",
        true,
    ),
    (
        "216a2a92-9ca3-4ca3-8af7-686c13b04290",
        "Shadowy Backstreet",
        false,
    ),
    (
        "840119bf-e60f-4ff7-9c9b-d420d09df545",
        "Underground Mortuary",
        true,
    ),
    (
        "37f924e1-7c25-4f06-88bb-054693a21e5a",
        "Conduit Pylons",
        false,
    ),
    (
        "382d18a2-438e-4ae7-a83f-1658ef1f9b07",
        "Hidden Grotto",
        true,
    ),
    (
        "a3648376-dc8b-409b-b2d1-c29e326a059c",
        "Surveillance Room",
        false,
    ),
];

/// "When this land enters, surveil 1."
///
/// The trigger is an ordinary one — it uses the stack, so the question
/// arrives as it resolves rather than on the way in, which is what tells it
/// apart from the `EnterModifier` clauses the same cards print.
#[test]
fn a_land_that_surveils_as_it_enters_looks_at_exactly_one_card() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, bin)) in ENTRY_SURVEIL_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 740 + u64::try_from(seed).expect("ten rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let library_before = library_size(&engine, p0);
        let top_before = engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied()
            .expect("p0 has a library");
        play_land(&mut engine, p0, land);

        let (options, min, max) = surveil_offer(&mut engine, p0);
        assert_eq!(
            options,
            vec![top_before],
            "{name}: surveil 1 looks at exactly the top card"
        );
        assert_eq!((min, max), (0, 1), "{name}: any number of the one it saw");

        let answer = if *bin { vec![top_before] } else { Vec::new() };
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: answer })
            .expect("both answers are legal");
        if *bin {
            assert_eq!(
                library_size(&engine, p0),
                library_before - 1,
                "{name}: the card it binned left the library"
            );
            assert_eq!(
                in_graveyard(&engine, p0, forest()),
                Some(top_before),
                "{name}: and reached the graveyard, which is the whole \
                 difference from a scry"
            );
        } else {
            assert_eq!(
                library_size(&engine, p0),
                library_before,
                "{name}: keeping it moved nothing"
            );
            assert_eq!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Library(p0))
                    .last()
                    .copied(),
                Some(top_before),
                "{name}: and left it where it was, on top"
            );
        }
    }
}

/// Every land in the pool that surveils for a **cost**, with the number it
/// looks at and the basics that pay for it — one land per mana of the
/// printed cost, and the colours it names.
///
/// The fixture *is* the assertion about the cost. A transcoded `{2}{R}{W}`
/// that came out `{5}` or `{2}{G}{U}` would not be offered over this board
/// at all, and one that came out `{3}` would leave a mana floating, which
/// the test counts. None of that is visible over a board of twenty basics,
/// which is what this table replaced.
const COST_SURVEIL_LANDS: [(&str, &str, u8, &[ManaColor]); 12] = [
    (
        "a32e08fa-bea4-4ba9-a126-9bf0a91f67e2",
        "Fields of Strife",
        1,
        &[
            ManaColor::Red,
            ManaColor::White,
            ManaColor::Red,
            ManaColor::White,
        ],
    ),
    (
        "349ea6c7-6b3e-417f-b082-b712e2b1635b",
        "Forum of Amity",
        1,
        &[
            ManaColor::White,
            ManaColor::Black,
            ManaColor::White,
            ManaColor::Black,
        ],
    ),
    (
        "4eb428ab-f5b0-46ca-98dd-b3466a91ef97",
        "Kishla Village",
        2,
        &[ManaColor::Green; 4],
    ),
    (
        "676141c3-a433-4aba-86fb-729628f96dfa",
        "Ominous Asylum",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "638ff242-63d5-457d-a7a6-40ad51052e2e",
        "Paradox Gardens",
        1,
        &[
            ManaColor::Green,
            ManaColor::Blue,
            ManaColor::Green,
            ManaColor::Blue,
        ],
    ),
    (
        "1af15c1d-a41c-44cc-9614-d72694dd26e8",
        "Savage Mansion",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "80f08b47-a237-4efd-8d86-dfe35a816b0e",
        "Sinister Hideout",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "33a4e73d-d93a-4b6f-88ff-cd53f20d178c",
        "Spectacle Summit",
        1,
        &[
            ManaColor::Blue,
            ManaColor::Red,
            ManaColor::Blue,
            ManaColor::Red,
        ],
    ),
    (
        "6ef30340-a26d-49aa-bc86-0b8aa5252f87",
        "Suburban Sanctuary",
        1,
        &[ManaColor::Green; 4],
    ),
    (
        "595f0eb5-f521-4174-9c48-b89e85ea907c",
        "Titan's Grave",
        1,
        &[
            ManaColor::Black,
            ManaColor::Green,
            ManaColor::Black,
            ManaColor::Green,
        ],
    ),
    (
        "a91f93fd-e428-4a36-b1b3-604b47a34287",
        "Tocasia's Dig Site",
        1,
        &[ManaColor::Green; 3],
    ),
    (
        "98e547de-b963-4ee4-9a08-67bae010734b",
        "University Campus",
        1,
        &[ManaColor::Green; 4],
    ),
];

/// The basic land that taps for one colour.
fn basic_of(color: ManaColor) -> CardIndex {
    let slot = match color {
        ManaColor::White => 0,
        ManaColor::Blue => 1,
        ManaColor::Black => 2,
        ManaColor::Red => 3,
        ManaColor::Green => 4,
        ManaColor::Colorless => unreachable!("no basic taps for colorless"),
    };
    baylee_cards::decks::basic_lands()[slot].expect("the pool has all five basics")
}

/// "{2}{R}{W}, {T}: Surveil 1." — the same effect reached through an
/// activated ability, which is a different path: it goes on the stack, its
/// `{T}` is part of the cost rather than the land's own mana tap, and the
/// question only arrives once it resolves.
///
/// The board is the land plus exactly the basics its cost names, and every
/// one of them is tapped before the ability is pressed — so the cost is
/// bounded from both sides: too expensive and the ability is never offered,
/// too cheap and a mana is left floating. Kishla Village is the row that
/// makes the amount worth carrying: it is the pool's only surveil 2, and a
/// reader that had defaulted to 1 would pass every other row.
#[test]
fn a_land_that_surveils_for_a_cost_looks_at_the_number_it_prints() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, amount, pays)) in COST_SURVEIL_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 760 + u64::try_from(seed).expect("twelve rows");
        let mut field = vec![land];
        field.extend(pays.iter().copied().map(basic_of));
        let mut engine = Duel::new(seed, forest()).battlefield(0, &field).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let object = on_battlefield(&engine, p0, land).expect("the land was seated");
        let library_before = library_size(&engine, p0);
        let graves_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
        // Everything but the land itself: its own `{T}` belongs to the cost
        // of the ability under test, not to paying for it.
        tap_mana_except(&mut engine, p0, object);
        // Ability 0 is the printed mana tap; ability 1 is the surveil.
        activate(&mut engine, p0, land, 1);

        let (options, _, max) = surveil_offer(&mut engine, p0);
        assert_eq!(
            options.len(),
            *amount as usize,
            "{name}: surveil {amount} looks at {amount} card(s)"
        );
        assert_eq!(max, *amount, "{name}: and may bin all of them");
        assert!(
            is_tapped(&engine, object),
            "{name}: the {{T}} in the cost was paid by this land"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "{name}: the cost spent every mana the board could make, so it is \
             the cost the card prints and not a cheaper one"
        );

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.clone(),
                },
            )
            .expect("binning everything it looked at");
        assert_eq!(
            library_size(&engine, p0),
            library_before - *amount as usize,
            "{name}: the cards it binned left the library"
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
            graves_before + *amount as usize,
            "{name}: and arrived in the graveyard, all of them"
        );
    }
}

/// The ten lands that print "sacrifice it unless you return a land you
/// control to its owner's hand", and the basic each one will take.
///
/// A table rather than ten tests, because ten tests would be one test
/// retyped: what differs between a Karoo and a Rith's Grove is a subtype in
/// a filter, and the rule under all ten is one transcoding. What the table
/// buys is the population — a card added to this family by a later codegen
/// run and *not* added here is caught by
/// [`every_land_that_pays_by_returning_one_is_in_the_table`], which asks the
/// compiled pool rather than this list.
///
/// The Lairs take any land at all (their filter excludes other Lairs, which
/// a Plains is not); the five Karoos each demand their own untapped basic,
/// which is why the payment is named per row and never assumed.
const PAYS_BY_RETURNING_A_LAND: &[(&str, &str)] = &[
    // Karoo cycle: "an untapped <basic> you control".
    ("d4e875d9-2245-470d-aa2f-1dfe66ce2d15", "Plains"), // Karoo
    ("3f347ebf-e0d2-4ae0-ad84-df7a460404e0", "Island"), // Coral Atoll
    ("38ba1956-5505-4a7a-b6af-e75715b1401f", "Mountain"), // Dormant Volcano
    ("ef3b8b0c-cea7-4bae-934c-9c65fd64245d", "Swamp"),  // Everglades
    ("f922f90a-b1a2-4630-9266-40726ca89f74", "Forest"), // Jungle Basin
    // Lair cycle: "a land you control" (that is not another Lair).
    ("e9a7dede-3968-4b0e-a707-419d46a6fec9", "Plains"), // Crosis's Catacombs
    ("19b58ec9-bb88-4193-8ea8-c8f09ceec1ed", "Plains"), // Darigaaz's Caldera
    ("d8b57707-796d-4488-8f91-65bb75bc6281", "Plains"), // Dromar's Cavern
    ("e13289e5-370b-435b-a38e-cf57c3078cec", "Plains"), // Rith's Grove
    ("7b2c7758-2b89-49ff-8838-8dc9880c7209", "Plains"), // Treva's Ruins
];

/// The cards that pay the same way and are **not** lands.
///
/// The transcoder writes `PlayerMayPayCostOr { cost: ReturnToHand(…) }`
/// wherever the reference writes an `UnlessCost$` that returns a permanent,
/// and nothing in that rule is about lands — which is why the sweep below
/// asks the whole pool and not `cards/lands/`. Waterspout Djinn prints the
/// Karoo clause on an **upkeep** trigger, so the driver above, which plays a
/// land and waits for it to arrive, could never reach it.
///
/// A row here carries the same promise a row in the table above does — that
/// some test plays the card — and differs only in which test that is.
const PAYS_BY_RETURNING_A_LAND_ELSEWHERE: &[(&str, &str)] = &[(
    "050dac46-9ba0-4b8a-b61b-1c7ec6f3723a",
    // played by `creatures::a_djinn_that_costs_a_bounce_pays_it_or_is_sacrificed`
    "Waterspout Djinn",
)];

/// The basic the row names, as a handle.
fn basic(name: &str) -> CardIndex {
    match name {
        "Plains" => plains(),
        "Island" => island(),
        "Mountain" => mountain(),
        "Swamp" => swamp(),
        "Forest" => forest(),
        other => panic!("no handle for {other}"),
    }
}

/// Passes priority until the land's enters-trigger has resolved far enough
/// to put its question up, and hands back what it asked.
///
/// `None` is the other legitimate outcome and is recognised by the **land**
/// rather than by the shape of the next question: a trigger that found
/// nothing to ask sacrifices the land and hands priority straight back, so
/// what says "there was no question" is that the land has left the
/// battlefield. Reading it off the pending instead walked on to
/// `ChooseAttackers` and panicked there.
fn reach_the_unless_question(
    engine: &mut Engine<RegistryLookup>,
    land: ObjectId,
) -> Option<(Vec<ObjectId>, ChoicePrompt)> {
    for _ in 0..8 {
        if let Pending::ChooseCards {
            options, prompt, ..
        } = engine.pending().clone()
        {
            return Some((options, prompt));
        }
        if !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land)
        {
            return None;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the trigger: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    None
}

/// Every one of the ten, played and paid for.
///
/// The land stays, and the land that paid goes to its owner's hand — the
/// two halves of one sentence, and the second is the one a `Sacrifice` that
/// merely did nothing would also pass.
#[test]
fn a_land_that_costs_a_bounce_keeps_itself_when_the_bounce_is_paid() {
    for (i, (oracle, pays)) in PAYS_BY_RETURNING_A_LAND.iter().enumerate() {
        let card = card_index(oracle);
        let payment = basic(pays);
        let p0 = PlayerId::new(0);
        let seed = 900 + u64::try_from(i).expect("ten rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[payment])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        let (options, prompt) =
            reach_the_unless_question(&mut engine, land).expect("the land asks what pays");
        assert_eq!(
            prompt,
            ChoicePrompt::CostReturn,
            "the price is a bounce, and the client draws the question from the prompt"
        );
        assert_eq!(
            options.len(),
            1,
            "the one land that matches the filter is the whole menu"
        );

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.clone(),
                },
            )
            .unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "the land was paid for and stays"
        );
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(&options[0]),
            "what paid goes to its owner's hand (CR 400.3)"
        );
    }
}

/// The same question, declined: naming nothing is how a player says no, and
/// what follows is the sacrifice the card prints.
///
/// `min: 0` is the whole of that — an empty answer has to be *legal*, or the
/// only way out of the question would be to pay. Written against Karoo
/// because one row is enough for the branch: the fallback is the same
/// `Effect::SacrificeSelf` on all ten.
#[test]
fn a_land_that_costs_a_bounce_sacrifices_itself_when_the_player_declines() {
    let p0 = PlayerId::new(0);
    let karoo = card_index("d4e875d9-2245-470d-aa2f-1dfe66ce2d15");
    let mut engine = Duel::new(921, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[karoo])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, karoo);
    let (options, _) =
        reach_the_unless_question(&mut engine, land).expect("the land asks what pays");
    let plain = options[0];

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "declining sacrifices the land"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&plain),
        "and the Plains that was offered stays where it was"
    );
}

/// Nobody can pay, so nobody is asked.
///
/// The Plains is **tapped**, which is the word the Karoo cycle prints and
/// the one half of its filter a test on an empty board would not reach: a
/// land on the battlefield that cannot pay is a menu with nothing on it, and
/// the engine runs the fallback without putting a question up at all. A
/// prompt with no legal answer would be a dead end for a client.
#[test]
fn a_land_that_costs_a_bounce_nobody_can_pay_asks_nothing() {
    let p0 = PlayerId::new(0);
    let karoo = card_index("d4e875d9-2245-470d-aa2f-1dfe66ce2d15");
    let mut engine = Duel::new(922, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[karoo])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Tap the Plains for mana, which is exactly how a player arrives here.
    let plain = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == plains())
        })
        .expect("the Plains is on the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plain })
        .unwrap();

    let land = play_land(&mut engine, p0, karoo);
    assert!(
        reach_the_unless_question(&mut engine, land).is_none(),
        "an untapped Plains is what the card asks for, and there is none"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "so the land sacrifices itself"
    );
}

/// The Battlebond "crowd" lands: "enters tapped unless you have two or more
/// opponents".
const UNTAPPED_WITH_A_CROWD: &[(&str, &str)] = &[
    (
        "761cb262-f83b-4a99-9345-b773182a7671",
        "Bountiful Promenade",
    ),
    ("819e1765-8325-4e6f-89c1-63ea86de369f", "Luxury Suite"),
    ("bd004c9d-771e-4e63-a97d-a2259c096af8", "Morphic Pool"),
    (
        "d1620449-930a-4895-a143-fd2a0a3c8b17",
        "Rejuvenating Springs",
    ),
    ("672e190d-8ea0-4a2e-b74f-5d35304631e4", "Sea of Clouds"),
    ("cf6d10ed-85c3-48f2-8ba0-2960e03b408b", "Spectator Seating"),
    ("45fe016e-1a09-410c-bbe3-4663ba06c5b7", "Spire Garden"),
    ("e3570ac7-c593-40e3-bbd6-ec3da6d8158d", "Training Center"),
    (
        "7c69f718-acc8-4851-8e5d-0cbaaa86192c",
        "Undergrowth Stadium",
    ),
    ("ebc5ac83-08d4-4d6b-b840-0c4ba71a38ab", "Vault of Champions"),
];

/// The Duskmourn "unlucky" lands: "enters tapped unless a player has 13 or
/// less life".
const UNTAPPED_WHEN_SOMEONE_IS_LOW: &[(&str, &str)] = &[
    (
        "0eec9984-cd11-4a52-9234-469c6a5fb9aa",
        "Abandoned Campground",
    ),
    ("47b6d2ae-d3d7-41eb-9172-2076eb8d028d", "Bleeding Woods"),
    ("6ccca5c2-66c3-495a-8d9e-1a9805569e52", "Etched Cornfield"),
    ("c56cd2ec-5907-4282-9162-d93b7dfd63b5", "Lakeside Shack"),
    ("c2cdefeb-3176-4faf-be54-a62d31f777a5", "Murky Sewer"),
    ("c8c632ab-14ec-44e1-ac00-81d48336320d", "Neglected Manor"),
    (
        "d55f7e20-11c6-44e2-8a21-dca67d3dbc68",
        "Peculiar Lighthouse",
    ),
    ("24a97436-ba61-4ebc-a560-a6c027ccfdf3", "Raucous Carnival"),
    ("8f69bd3a-244e-42d8-bfac-5a426f4b54b4", "Razortrap Gorge"),
    ("3a5b3405-a1e3-4aad-ab4e-1b8db2d1f3a8", "Strangled Cemetery"),
];

/// Whether the land `card` arrives tapped on the board `build` sets up.
fn arrives_tapped(build: impl FnOnce() -> crate::engine::testkit::Duel, card: CardIndex) -> bool {
    let p0 = PlayerId::new(0);
    let mut engine = build().hand(0, &[card]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, card);
    engine
        .state()
        .object(land)
        .expect("the land arrived")
        .status
        .contains(Status::TAPPED)
}

/// All ten crowd lands, at the two table sizes their sentence is about.
///
/// A duel is the board every other land test in this file runs on, and it is
/// the one board this cycle cannot be measured from: with a single opponent
/// every one of the ten enters tapped, so a test written there would assert
/// the same branch twice and pass whatever the rule did.
#[test]
fn a_crowd_land_wants_two_opponents_and_counts_only_opponents() {
    for (i, (oracle, name)) in UNTAPPED_WITH_A_CROWD.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 980 + u64::try_from(i).expect("ten rows");
        assert!(
            arrives_tapped(|| Duel::new(seed, forest()), card),
            "{name} entered untapped in a duel, where you have one opponent"
        );
        assert!(
            !arrives_tapped(|| Duel::table(seed, forest(), 3), card),
            "{name} entered tapped at a three-player table"
        );
    }

    // The half no seat count can show: two other players, one of them a
    // teammate, is **one** opponent. A rule that counted seats rather than
    // asking `is_opponent` passes every line above and fails this one.
    let suite = card_index("819e1765-8325-4e6f-89c1-63ea86de369f");
    assert!(
        arrives_tapped(
            || Duel::table(991, forest(), 3)
                .team(0, 1)
                .team(1, 1)
                .team(2, 2),
            suite
        ),
        "Luxury Suite counted a teammate as an opponent"
    );
    assert!(
        !arrives_tapped(
            || Duel::table(992, forest(), 3)
                .team(0, 1)
                .team(1, 2)
                .team(2, 3),
            suite
        ),
        "three seats on three sides is two opponents, and the land should be untapped"
    );
}

/// All ten unlucky lands, and the three life totals their sentence reads.
///
/// Both seats start at twenty, so the duel that every other land test uses
/// only ever shows the tapped branch here too. What turns the land on is
/// **a** player at thirteen or less — including its own controller, which is
/// the reading a test written only against the opponent would never separate
/// from "an opponent has 13 or less life", a different card.
#[test]
fn an_unlucky_land_reads_every_life_total_including_its_own() {
    for (i, (oracle, name)) in UNTAPPED_WHEN_SOMEONE_IS_LOW.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 1000 + u64::try_from(i).expect("ten rows");
        assert!(
            arrives_tapped(|| Duel::new(seed, forest()), card),
            "{name} entered untapped with both seats at twenty"
        );
        assert!(
            !arrives_tapped(|| Duel::new(seed, forest()).life(1, 13), card),
            "{name} entered tapped with an opponent at exactly thirteen"
        );
        assert!(
            !arrives_tapped(|| Duel::new(seed, forest()).life(0, 12), card),
            "{name} ignored its own controller's life total"
        );
    }

    // Thirteen is the boundary the card prints, so fourteen is the other
    // side of it — without this the whole cycle would pass with a `<` for a
    // `<=` and be wrong on exactly one life total.
    let gorge = card_index("8f69bd3a-244e-42d8-bfac-5a426f4b54b4");
    assert!(
        arrives_tapped(|| Duel::new(1020, forest()).life(1, 14), gorge),
        "fourteen is not thirteen or less"
    );
}

/// The Ravnica bounce lands: "when this land enters, return a land you
/// control to its owner's hand".
///
/// The other Karoo sentence, and the difference is who is out of pocket.
/// The cycle above charges a bounce as a *price* and sacrifices the land
/// when it goes unpaid; these eleven simply do it, which is why the effect
/// is `ReturnChosenToHand` and not a `CostPart` — there is nothing to
/// decline.
const RETURNS_A_LAND_YOU_CONTROL: &[(&str, &str)] = &[
    ("189fc8f4-17ac-4f1d-82c8-8401445bdaf4", "Azorius Chancery"),
    ("8fa3ac81-3dfe-4565-be99-5554f7597b4b", "Boros Garrison"),
    ("378a1d57-e2f1-4b84-9692-1564602e9e99", "Dimir Aqueduct"),
    ("1b301478-b14f-4ef8-94e6-9647d582eabe", "Golgari Rot Farm"),
    ("657243dd-e479-4f4b-99d2-09b55d833a35", "Gruul Turf"),
    ("ee723c7c-ec9f-4ffb-8f36-cd7637eb1fae", "Guildless Commons"),
    ("1cb9d94a-3039-4f2e-8fcc-6996f9a45f74", "Izzet Boilerworks"),
    ("aa00ae0b-7c0f-427e-8102-ce0e2a6af5df", "Orzhov Basilica"),
    ("0a023964-2905-4928-9c3e-dc63e6ebd218", "Rakdos Carnarium"),
    ("00ef1c55-dea1-4564-bd57-66de86cba4df", "Selesnya Sanctuary"),
    (
        "046f5783-cc7b-416a-8cf6-2bcef9c2cc1a",
        "Simic Growth Chamber",
    ),
];

/// Every one of the eleven, played and answered.
///
/// Two lands are on the menu and not one, because **the bounce land itself
/// is a legal answer** — it is a land its controller controls, and nothing
/// in the printed sentence excludes it. The count is asserted rather than
/// the contents for exactly that reason: a reader that helpfully left the
/// source out would still return the Forest and pass every other line here.
#[test]
fn a_bounce_land_returns_a_land_its_controller_chooses() {
    for (i, (oracle, name)) in RETURNS_A_LAND_YOU_CONTROL.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 960 + u64::try_from(i).expect("eleven rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        let (options, prompt) = reach_the_unless_question(&mut engine, land)
            .expect("{name} asks which land comes back");
        assert_eq!(
            prompt,
            ChoicePrompt::Generic,
            "{name} is not paying for anything, so the question is not a cost"
        );
        assert_eq!(
            options.len(),
            2,
            "{name} and the Forest are both lands p0 controls"
        );
        let forest_on_board = *options
            .iter()
            .find(|id| **id != land)
            .expect("the Forest is the other option");

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![forest_on_board],
                },
            )
            .unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(&forest_on_board),
            "{name} put the chosen land in its owner's hand (CR 400.3)"
        );
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "{name} stays: it returns a land, it does not sacrifice itself"
        );
    }
}

/// The bounce land alone on the battlefield returns **itself**.
///
/// One row is enough because the rule is the same on all eleven, and this is
/// the board that says the menu is not filtered down to "some other land":
/// with nothing else in play the only legal answer is the source, the player
/// is still asked, and the land goes back to the hand it was just played
/// from. An engine that excluded the source would have to ask an empty
/// question here, which is the dead end `min: 1` forbids.
#[test]
fn a_bounce_land_alone_on_the_battlefield_returns_itself() {
    let p0 = PlayerId::new(0);
    let chancery = card_index("189fc8f4-17ac-4f1d-82c8-8401445bdaf4");
    let mut engine = Duel::new(971, forest()).hand(0, &[chancery]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, chancery);
    let (options, _) =
        reach_the_unless_question(&mut engine, land).expect("the land asks even with one answer");
    assert_eq!(options, vec![land], "the source is the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&land),
        "it returned itself to its owner's hand"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "and is no longer on the battlefield"
    );
}

/// The four lands that charge *mana* for the same escape, and the CR 605.3a
/// window they need.
///
/// One board and two answers per card: the player is asked with an empty
/// pool, makes the mana inside the window, and keeps the land — then the
/// same card, declined, is sacrificed. Rupture Spire charges `{1}` and the
/// other three charge `{1}` as well, so one untapped Forest is the whole
/// price.
const PAYS_WITH_MANA: &[&str] = &[
    "a6543f71-0326-4e1f-b58f-9ce325d5d036", // Gateway Plaza
    "69c63055-ed44-4b32-b591-f3c6c2f3e7d1", // Archway Commons
    "7eadffcb-1e15-44c1-b1db-78c71b8ec1ce", // Rupture Spire
    "98334bfa-c516-4c20-bdc5-9e32e7127adc", // Transguild Promenade
];

#[test]
fn a_land_that_costs_mana_is_kept_by_making_it_inside_the_window() {
    for (i, oracle) in PAYS_WITH_MANA.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 940 + u64::try_from(i).expect("four rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        for _ in 0..8 {
            match engine.pending().clone() {
                Pending::YesNo { .. } => break,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected while waiting for the tax: {other:?}"),
            }
        }
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            panic!("expected the tax question, got {:?}", engine.pending())
        };
        assert_eq!(player, p0);
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "the question is put with an empty pool — CR 605.3a is what makes that answerable"
        );

        // Yes, with nothing floating: the engine opens the window.
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        let forest_id = *engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .find(|id| {
                engine
                    .state()
                    .object(**id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == forest())
            })
            .expect("the Forest is on the battlefield");
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: forest_id })
            .unwrap();
        engine.apply(p0, PlayerAction::PassPriority).unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "the tax was paid out of mana made inside the window"
        );
    }
}

/// The two tables above are the whole family, asked of the compiled pool.
///
/// [`PAYS_BY_RETURNING_A_LAND`] and [`PAYS_BY_RETURNING_A_LAND_ELSEWHERE`]
/// claim between them to be every card that escapes an effect by bouncing a
/// land, and a claim about a population is worth what a test says it is: a
/// fifteenth of these written by a later codegen run would otherwise pass
/// every gate in this file while never being played once. The
/// same argument as `lints::every_layer_in_the_pool_is_the_one_its_modifier_derives`
/// and the reason `no_card_claims_a_keyword_the_engine_ignores` is a test.
///
/// Set equality both ways, because the two failures are different repairs: a
/// card the pool has and the table does not is a card to play, and a row
/// naming a card the pool no longer has is a row to delete.
#[test]
fn every_land_that_pays_by_returning_one_is_in_the_table() {
    let mut pool: Vec<&str> = Vec::new();
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let faces = def.faces.iter().map(|f| f.abilities);
        for list in core::iter::once(def.abilities).chain(faces) {
            let dump = format!("{list:?}");
            if dump.match_indices("PlayerMayPayCostOr { ").any(|(at, _)| {
                dump[at..]
                    .split_once("cost: ")
                    .is_some_and(|(_, tail)| tail.starts_with("ReturnToHand("))
            }) {
                pool.push(oracle_id);
            }
        }
    }
    pool.sort_unstable();
    pool.dedup();
    let mut table: Vec<&str> = PAYS_BY_RETURNING_A_LAND
        .iter()
        .chain(PAYS_BY_RETURNING_A_LAND_ELSEWHERE)
        .map(|(id, _)| *id)
        .collect();
    table.sort_unstable();

    assert!(
        pool.len() >= 11,
        "only {} cards in the pool escape a sacrifice by bouncing a land, \
         against the eleven that carried it when this was written — the \
         probe broke",
        pool.len()
    );
    assert_eq!(
        pool, table,
        "the pool and PAYS_BY_RETURNING_A_LAND disagree. A card the table \
         is missing is a card nothing plays; a row the pool is missing names \
         a card that left"
    );
}

/// The eleven lands whose *activation* cost asks the player to name an
/// object, with the one thing each needs on the board to pay it.
///
/// A second family from the same transcoder rule as
/// [`PAYS_BY_RETURNING_A_LAND`] and a different sentence: this is not an
/// escape from an effect but a price on an ability, `{T}, Sacrifice a
/// creature:` and its neighbours. What they share is the part the player
/// answers by naming something, which is what this table plays.
///
/// Two cards feed all of them and are not chosen for convenience. Baleful
/// Strix is an *artifact creature — Bird*, so one card is a legal answer to
/// "a creature", "an artifact" and "a Bird" alike; Gateway Plaza is a
/// *Gate*, which is a land. Ipnu Rivulet needs neither, because it
/// sacrifices a Desert and is one — the row a filter written as "another"
/// would have got wrong.
const PAYS_BY_NAMING_AN_OBJECT: &[(&str, Feed)] = &[
    ("86fb3749-37d6-48a6-8524-71e996850307", Feed::Strix), // High Market
    ("5effaa94-7f87-4485-8959-473d584c5034", Feed::Strix), // Grim Backwoods
    ("ea4d6fcd-21e0-4e9f-b406-a89042998d98", Feed::Strix), // Keldon Necropolis
    ("b6cc062c-eb39-46ee-bd6d-17f1db0ac50d", Feed::Strix), // Phyrexia's Core
    ("4adc39dd-8de1-4298-947c-ff666ec3adeb", Feed::Strix), // Seaside Haven
    ("9abf9a0e-8e7d-406b-a01d-d4870b30134e", Feed::Strix), // The Shire
    ("d3df7128-31dd-4d71-90be-87e2e9ff51b4", Feed::Plaza), // Dust Bowl
    ("e10e84a7-d564-487a-ac64-5a001a45ee90", Feed::Plaza), // Rath's Edge
    ("35922a30-6b84-44dd-a2f0-306554a1ae90", Feed::Plaza), // Heap Gate
    ("c17d799f-adc9-4c41-87cf-b243b5ea3be1", Feed::Itself), // Ipnu Rivulet
    ("850bb6f7-48d3-4d65-9220-b0bec5ee6b64", Feed::HandCard), // Fogwell's Gym
];

/// What a row seats so its land can pay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Feed {
    /// Baleful Strix: a creature, an artifact and a Bird in one card.
    Strix,
    /// Gateway Plaza: a land, and a Gate.
    Plaza,
    /// A card in hand, for the one that discards.
    HandCard,
    /// The land is its own feed.
    Itself,
}

fn baleful_strix() -> CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}

fn gateway_plaza() -> CardIndex {
    card_index("a6543f71-0326-4e1f-b58f-9ce325d5d036")
}

/// The `Cost` an activated ability charges, both spellings.
///
/// `ActivatedConditional` is the twin six readers across this workspace have
/// already been found matching only half of.
fn activation_cost(ability: &'static AbilityDef) -> Option<&'static baylee_cards_dsl::Cost> {
    match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            Some(cost)
        }
        _ => None,
    }
}

/// The ability of `card` whose cost asks the player to name an object.
///
/// Found by reading the compiled `CostPart`s rather than by writing an index
/// into the table: the index is a card file's ability order, which is
/// codegen's to change, and a test pinned to it would start exercising the
/// mana ability the day a card grew a second one.
fn ability_that_asks(engine: &Engine<RegistryLookup>, card: CardIndex) -> Option<(ObjectId, u32)> {
    use baylee_cards_dsl::CostPart;
    let Pending::Priority { legal, .. } = engine.pending() else {
        return None;
    };
    legal.abilities.iter().copied().find(|(id, index)| {
        engine
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .filter(|c| c.index == card)
            .and_then(|c| baylee_cards::by_index(c.index))
            .and_then(|def| def.abilities.get(*index as usize))
            .and_then(activation_cost)
            .is_some_and(|cost| {
                cost.parts.iter().any(|part| {
                    matches!(
                        part,
                        CostPart::Sacrifice(_)
                            | CostPart::Discard(_)
                            | CostPart::TapOther(_)
                            | CostPart::ReturnToHand(_)
                    )
                })
            })
    })
}

/// Every row: the land is played, the price is named, and what was named
/// has paid.
///
/// The assertion is on the **object**, not on the ability's effect: what
/// this rule wrote is the cost, and a sacrifice that drew a card while
/// leaving the creature on the battlefield is exactly the failure a test on
/// "did I draw" would pass. Which zone the object lands in is read off the
/// part it paid, because a discard and a sacrifice both reach a graveyard
/// and a tap reaches nothing at all.
#[test]
fn a_land_whose_cost_names_an_object_is_paid_with_that_object() {
    for (i, (oracle, feed)) in PAYS_BY_NAMING_AN_OBJECT.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 960 + u64::try_from(i).expect("eleven rows");
        // Enough of every colour for the dearest of them, `{4}{R}`.
        let mut board = vec![
            plains(),
            plains(),
            island(),
            island(),
            swamp(),
            swamp(),
            mountain(),
            mountain(),
            forest(),
            forest(),
        ];
        let mut hand = vec![card];
        match feed {
            Feed::Strix => board.push(baleful_strix()),
            Feed::Plaza => board.push(gateway_plaza()),
            Feed::HandCard => hand.push(plains()),
            Feed::Itself => {}
        }
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &board)
            .hand(0, &hand)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        // Round the turn before activating. Three of these lands come down
        // tapped — The Shire unless you control a legendary creature, and
        // it is not one — and their own `{T}` is part of the price, so a
        // test that activated the turn they arrived would be measuring
        // summoning-sick lands rather than costs.
        cross_into_the_next_own_main(&mut engine, p0);
        // Everything but the land itself: its `{T}` is part of the price.
        tap_all_mana_but(&mut engine, p0, Some(card));

        let (source, index) = ability_that_asks(&engine, card)
            .unwrap_or_else(|| panic!("{oracle} offers no ability that asks for an object"));
        assert_eq!(source, land, "the ability is on the land just played");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: index,
                },
            )
            .unwrap();

        // CR 601.2c puts targets before costs, so three of these rows ask
        // where the ability points before they ask what pays for it. The
        // first legal answer will do — this test is about the price.
        if let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = engine.pending().clone()
        {
            let (objects, players) = match (options.first(), player_options.first()) {
                (Some(&object), _) => (vec![object], vec![]),
                (None, Some(&player)) => (vec![], vec![player]),
                (None, None) => panic!("{oracle} asked for a target and offered none"),
            };
            engine
                .apply(p0, PlayerAction::ChooseTargets { objects, players })
                .unwrap();
        }
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "{oracle} asked no cost question, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(player, p0);
        assert_eq!(
            (min, max),
            (1, 1),
            "an activation cost is not optional — one object, and exactly one"
        );
        if matches!(feed, Feed::Strix | Feed::Itself) {
            no_basic_land_on_the_menu(&engine, &options, oracle);
        }
        let paid = *options.first().unwrap_or_else(|| {
            panic!("{oracle} put an empty menu up, which is a dead end for a client")
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![paid],
                },
            )
            .unwrap();

        the_object_paid(&engine, card, index, paid, p0, oracle);
    }
}

/// No basic land is a legal answer to a price that is not a land.
///
/// An independent reading of the menu, and the reason it is here: reading
/// only `options[0]` let a `cost_wizard::options` with its filter bypassed
/// pass the test above, because the first offer happened to be the right
/// one anyway. Ten basics are on that board precisely so that a menu which
/// ignored its filter would be visibly wrong.
fn no_basic_land_on_the_menu(engine: &Engine<RegistryLookup>, options: &[ObjectId], oracle: &str) {
    for &offered in options {
        let is_basic = engine
            .state()
            .object(offered)
            .and_then(|o| o.card)
            .and_then(|c| baylee_cards::by_index(c.index))
            .is_some_and(|def| {
                def.faces.first().is_some_and(|face| {
                    face.supertypes
                        .contains(baylee_core::types::SupertypeSet::BASIC)
                })
            });
        assert!(
            !is_basic,
            "{oracle} offers a basic land for a price that is not a land"
        );
    }
}

/// What paying looks like, read off the part that was paid.
///
/// A discard and a sacrifice both reach a graveyard and a tap reaches
/// nothing at all, so the outcome is asked of the `CostPart` rather than
/// assumed — and it is asked of the **object**, because a sacrifice that
/// drew its card while leaving the creature on the battlefield is exactly
/// what a test on the ability's effect would let through.
fn the_object_paid(
    engine: &Engine<RegistryLookup>,
    card: CardIndex,
    index: u32,
    paid: ObjectId,
    seat: PlayerId,
    oracle: &str,
) {
    use baylee_cards_dsl::CostPart;
    let part = baylee_cards::by_index(card)
        .and_then(|def| def.abilities.get(index as usize))
        .and_then(activation_cost)
        .and_then(|cost| {
            cost.parts.iter().find(|part| {
                matches!(
                    part,
                    CostPart::Sacrifice(_) | CostPart::Discard(_) | CostPart::TapOther(_)
                )
            })
        })
        .expect("the part this row is about");
    match part {
        CostPart::TapOther(_) => assert!(
            engine
                .state()
                .object(paid)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "{oracle}: what paid is tapped"
        ),
        _ => assert!(
            !engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&paid)
                && !engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(seat))
                    .contains(&paid),
            "{oracle}: what paid has left the zone it paid from"
        ),
    }
}

/// Command Bridge: "sacrifice it unless you **tap** an untapped permanent
/// you control."
///
/// The same "unless" as the Karoo cycle with the other asking part, and the
/// row that says the two families are one rule: the price is a `TapOther`
/// rather than a `ReturnToHand`, and nothing else about the sentence
/// changes. What it also pins is CR 118.3 on the menu — a permanent that is
/// already tapped is not an answer, which is what the card's own word
/// "untapped" says and what `cost_wizard::options` supplies.
#[test]
fn a_land_that_costs_a_tap_keeps_itself_when_something_untapped_is_named() {
    let p0 = PlayerId::new(0);
    let bridge = card_index("87c8e1ed-258a-4a89-bcc6-211405e49692");
    let mut engine = Duel::new(930, forest())
        .battlefield(0, &[forest(), island()])
        .hand(0, &[bridge])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // One of the two is spent, so the menu has to be shorter than the board.
    let island_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == island())
        })
        .expect("the Island is on the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: island_id })
        .unwrap();

    let land = play_land(&mut engine, p0, bridge);
    let (options, prompt) = reach_the_unless_question(&mut engine, land).expect("it asks");
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert!(
        !options.contains(&island_id),
        "a tapped permanent cannot be tapped to pay (CR 118.3)"
    );
    let paid = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .unwrap();

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "the land was paid for and stays"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "and what paid is tapped"
    );
}

/// Fountainport: "{2}, {T}, Sacrifice a **token**: Draw a card."
///
/// The thirteenth of the family and the only one whose price is a filter
/// over what a permanent *is* rather than what it is called: `Filter::IsToken`
/// is true of no card in any decklist, so the land has to make its own
/// payment first. Which is also why it takes two turns — one `{T}` per turn,
/// and this card charges one for the Treasure and one for the draw.
#[test]
fn a_land_that_sacrifices_a_token_makes_one_first() {
    let p0 = PlayerId::new(0);
    let port = card_index("94e8b0a9-44a1-4dce-8d44-78681ae638a1");
    let mut engine = Duel::new(931, forest())
        .battlefield(
            0,
            &[
                port,
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Turn one: `{4}, {T}: Create a Treasure token`.
    let treasure_maker = baylee_cards::by_index(port)
        .expect("the card is in the pool")
        .abilities
        .iter()
        .position(|a| {
            format!("{a:?}").contains("CreateToken") && !format!("{a:?}").contains("LoseLife")
        })
        .expect("the Treasure ability");
    tap_all_mana_but(&mut engine, p0, Some(port));
    let source = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == port)
        })
        .expect("Fountainport is on the battlefield");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: u32::try_from(treasure_maker).expect("a small index"),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| e.state().object(*id).is_some_and(|o| o.card.is_none()))
    });

    // Turn two: the token is on the board, so the sacrifice has an answer.
    cross_into_the_next_own_main(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(port));
    let (asks, index) = ability_that_asks(&engine, port).expect("the sacrifice is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: asks,
                ability_index: index,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the cost question, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        1,
        "the Treasure is a token and nothing else on this board is"
    );
    assert!(
        engine
            .state()
            .object(options[0])
            .is_some_and(|o| o.card.is_none()),
        "what the menu offers is the token"
    );
}

/// The eleven lands the transcoder writes out of a `ChangeZone` that reads a
/// library, with the basic each one's own filter accepts and one it refuses.
///
/// The second column fills the library, so there is something to find at
/// all. The third is what
/// [`a_land_that_searches_finds_nothing_outside_its_own_filter`] fills it
/// with instead, and is empty for the six that take any basic land there is
/// — those six have no outside.
const SEARCHES_THE_LIBRARY: &[(&str, &str, &str)] = &[
    // "a basic land card".
    ("861eb7d7-7616-4620-a4fd-4b8c3bf00dd1", "Forest", ""), // Promising Vein
    ("032b8a0d-491a-4a12-ab9f-689010054d5b", "Forest", ""), // Prismatic Vista
    ("619173f4-0403-49cd-9659-2fedd5028a90", "Forest", ""), // Shire Terrace
    ("58eaaa8b-45c6-439b-bdd1-5f4e77a75a8c", "Forest", ""), // Terminal Moraine
    ("6a7f3e1f-6798-4644-b64c-7765f81f0938", "Forest", ""), // Vibrant Cityscape
    ("543e6bb3-a867-43bf-a737-2f5d6d8dc631", "Forest", ""), // Warped Landscape
    // The Panorama cycle: three named basics each, and two it must refuse.
    ("0a1d817d-dce8-4e83-a380-909f7c9eee46", "Forest", "Swamp"), // Bant
    ("6b9cd3d0-4316-4945-b960-12f51052d260", "Plains", "Forest"), // Esper
    ("743f4488-fef1-4f4d-b745-d2de92423e00", "Island", "Plains"), // Grixis
    ("f39f33ac-074d-442d-ae4c-1d694ee315f3", "Swamp", "Plains"), // Jund
    ("71e28800-c42c-48c0-95e5-0296be54a4e8", "Mountain", "Island"), // Naya
];

/// The ability of `card` that reads a library, and where what it finds goes.
///
/// `finds` is read off the compiled card rather than written into the table
/// beside it, for the same reason [`ability_that_asks`] reads the cost: the
/// table would then be a second opinion about a card, and the card is the
/// one this rule wrote. Both activated spellings, because
/// `ActivatedConditional` is the twin readers keep missing.
fn ability_that_searches(
    engine: &Engine<RegistryLookup>,
    card: CardIndex,
) -> Option<(ObjectId, u32, &'static [Find])> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return None;
    };
    legal.abilities.iter().copied().find_map(|(id, index)| {
        let def = engine
            .state()
            .object(id)
            .and_then(|o| o.card)
            .filter(|c| c.index == card)
            .and_then(|c| baylee_cards::by_index(c.index))?;
        let effects = match def.abilities.get(index as usize)? {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. } => *effects,
            _ => return None,
        };
        effects.iter().find_map(|effect| match effect {
            Effect::SearchLibrary { finds, .. } => Some((id, index, *finds)),
            _ => None,
        })
    })
}

/// Passes priority until the search puts its question up.
///
/// `None` is the other legitimate outcome: a search that matches nothing in
/// the library shuffles and asks nobody (CR 701.23b), so what says "there
/// was no question" is the ability having left the stack. The land is no
/// signal here — it was sacrificed to *pay* for this, one step before the
/// ability ever went on the stack.
fn reach_the_search(engine: &mut Engine<RegistryLookup>) -> Option<(Vec<ObjectId>, u8, u8)> {
    for _ in 0..8 {
        if let Pending::ChooseCards {
            options,
            min,
            max,
            prompt: ChoicePrompt::SearchLibrary,
            ..
        } = engine.pending().clone()
        {
            return Some((options, min, max));
        }
        if engine.state().zones.list(ZoneLocation::Stack).is_empty() {
            return None;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the search: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    // Never a quiet `None`: one ability on an otherwise empty stack resolves
    // in two passes, so eight of them mean the walk lost its way — and the
    // negative test below reads `None` as "the search asked nothing", which
    // an exhausted loop would satisfy without ever reaching the search.
    panic!("the ability never resolved: {:?}", engine.pending())
}

/// Plays the row's land, pays for it, and hands back the engine standing on
/// whatever the search asked — with the land already gone, because every
/// one of these eleven sacrifices itself to pay.
fn a_land_that_searches(
    seed: u64,
    card: CardIndex,
    library: CardIndex,
) -> (Engine<RegistryLookup>, &'static [Find]) {
    let p0 = PlayerId::new(0);
    // Two of every basic: enough for the dearest of the eleven, `{2}`, and
    // colours the filters have opinions about.
    let mut engine = Duel::new(seed, library)
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, card);
    // Round the turn: three of these come down tapped, and every one of them
    // spends its own `{T}` as part of the price.
    cross_into_the_next_own_main(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(card));

    let (source, index, finds) = ability_that_searches(&engine, card)
        .expect("the land offers the ability that reads a library");
    assert_eq!(source, land, "the ability is on the land just played");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .unwrap();
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "the land pays for this with itself, and a cost is paid on activation"
    );
    (engine, finds)
}

/// Every row: the land is played, sacrificed for its own ability, and what
/// the search found is on the battlefield in the state the card prints.
///
/// The count and the tapping are read off the compiled `finds` rather than
/// asserted as constants, so the day a `Find` in one of these cards changes
/// the test follows the card instead of arguing with it. What is fixed here
/// is the sentence around them: the search offers exactly as many as it
/// finds, no fewer (none of the eleven prints "up to"), and every card named
/// arrives.
#[test]
fn a_land_that_searches_puts_what_it_found_onto_the_battlefield() {
    for (i, (oracle, fills, _)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 1020 + u64::try_from(i).expect("eleven rows");
        let (mut engine, finds) = a_land_that_searches(seed, card, basic(fills));

        let (options, min, max) =
            reach_the_search(&mut engine).unwrap_or_else(|| panic!("{oracle} asked nothing"));
        let want = u8::try_from(finds.len()).expect("a handful at most");
        assert_eq!(
            (min, max),
            (want, want),
            "{oracle} prints no \"up to\", so the search is for all {want} of them"
        );
        let chosen: Vec<ObjectId> = options.iter().copied().take(finds.len()).collect();
        assert_eq!(
            chosen.len(),
            finds.len(),
            "{oracle} was offered {} cards out of a library of sixty {fills}",
            options.len()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: chosen.clone(),
                },
            )
            .unwrap();

        let battlefield = engine.state().zones.list(ZoneLocation::Battlefield).clone();
        for (found, find) in chosen.iter().zip(finds) {
            assert_eq!(
                find.dest,
                SearchDest::Battlefield,
                "{oracle} is one of the eleven that fetch onto the battlefield"
            );
            assert!(
                battlefield.contains(found),
                "{oracle} named a card and it never arrived"
            );
            assert_eq!(
                engine
                    .state()
                    .object(*found)
                    .and_then(|o| o.card)
                    .map(|c| c.index),
                Some(basic(fills)),
                "{oracle} put something other than the {fills} it was handed onto the battlefield"
            );
            assert_eq!(
                entered_tapped(&engine, *found),
                find.tapped,
                "{oracle} prints tapped = {}, and the card arrived the other way",
                find.tapped
            );
        }
    }
}

/// The five Panoramas, over a library of the one basic each of them refuses.
///
/// The independent reading of the filter, and the reason it is a second
/// test: the test above fills the library with sixty cards the filter
/// accepts, so a `SearchLibrary` that ignored its filter entirely would
/// pass it every time. Here nothing matches, the search asks no question at
/// all (CR 701.23b), and the land is still gone — a filter that let the
/// wrong basic through would put a question up instead.
#[test]
fn a_land_that_searches_finds_nothing_outside_its_own_filter() {
    let mut checked = 0;
    for (i, (oracle, _, refuses)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        if refuses.is_empty() {
            continue;
        }
        checked += 1;
        let card = card_index(oracle);
        let seed = 1040 + u64::try_from(i).expect("eleven rows");
        let (mut engine, _) = a_land_that_searches(seed, card, basic(refuses));
        assert!(
            reach_the_search(&mut engine).is_none(),
            "{oracle} offered a search over a library of sixty {refuses}, which it does not name"
        );
    }
    assert_eq!(
        checked, 5,
        "the Panorama cycle is five cards, and this test speaks for all of them"
    );
}

/// The ten fast lands, on the two boards their sentence divides.
///
/// Every other enters-tapped cycle in this file is a lower bound, and these
/// are the upper one — so the board that turns a slow land on is the board
/// that turns these off, and a test written on a duel's empty table would
/// show only the untapped branch of all of them.
const UNTAPPED_ON_A_SMALL_BOARD: &[(&str, &str)] = &[
    ("5ad94412-6f79-4c5d-bbd4-4ef5779a7b6d", "Blackcleave Cliffs"),
    ("66fa2326-1b5d-41fb-b919-83bf9f383577", "Blooming Marsh"),
    ("88f8f683-738e-48f3-afff-c8f73f1033a2", "Botanical Sanctum"),
    (
        "2d899466-b1eb-4901-b626-1f2fb09b786d",
        "Concealed Courtyard",
    ),
    ("a05f641c-15c9-43dc-ae0d-1ea372fd33d5", "Copperline Gorge"),
    ("a2b48695-f7d7-42ce-a8a0-2a723428542a", "Darkslick Shores"),
    ("3f17c60e-923a-4392-9da8-87d9ded009b7", "Inspiring Vantage"),
    ("94f6c407-e665-4032-be13-a01e40c1f306", "Razorverge Thicket"),
    ("9e7a240d-dc33-47ac-9f17-77fab4c1c340", "Seachrome Coast"),
    ("eb0d8093-5f93-4b25-9384-08f9731bfb28", "Spirebluff Canal"),
];

/// "Enters tapped unless you control two or fewer **other** lands."
///
/// Three assertions, and each is a different way to get the sentence wrong.
/// Two other lands plus the land itself is three on the battlefield and it
/// comes down untapped — the case a count that included the entering land
/// taps, and the only boundary that separates "other" from "any". Three
/// others taps it, which is the bound being a bound rather than a decoration.
/// And three of them across the table changes nothing, because the card says
/// "you control".
#[test]
fn a_fast_land_is_untapped_while_your_own_board_is_small() {
    for (i, (oracle, name)) in UNTAPPED_ON_A_SMALL_BOARD.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 1040 + u64::try_from(i).expect("ten rows");
        assert!(
            !arrives_tapped(
                || Duel::new(seed, forest()).battlefield(0, &[forest(), forest()]),
                card
            ),
            "{name} entered tapped over two other lands, and two or fewer is its own sentence"
        );
        assert!(
            arrives_tapped(
                || Duel::new(seed, forest()).battlefield(0, &[forest(), forest(), forest()]),
                card
            ),
            "{name} entered untapped over three other lands"
        );
        assert!(
            !arrives_tapped(
                || Duel::new(seed, forest()).battlefield(1, &[forest(), forest(), forest()]),
                card
            ),
            "{name} counted the lands across the table, and its sentence says \"you control\""
        );
    }
}

/// Cave of the Frost Dragon prints the same bound as its complement — "if
/// you control two or more other lands, this land enters tapped" — which is
/// one lower, and that off-by-one is the whole reading.
///
/// The eleventh card the upper bound finished, and the only one of the five
/// manlands whose animation the transcoder could also read: the other four
/// print a ward cost or a trigger no rule says. So it is played twice here,
/// once for the bound and once for the sentence underneath it, because a
/// card that arrives correctly and animates into the wrong thing is still a
/// wrong card.
#[test]
fn a_manland_bound_is_one_lower_and_its_dragon_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let cave = card_index("1e4146d2-cfa0-4f5e-9761-3c83519b90c3");
    assert!(
        !arrives_tapped(
            || Duel::new(1060, forest()).battlefield(0, &[forest()]),
            cave
        ),
        "one other land is not two, and the Cave should have entered untapped"
    );
    assert!(
        arrives_tapped(
            || Duel::new(1061, forest()).battlefield(0, &[forest(), forest()]),
            cave
        ),
        "two other lands is exactly what the card names"
    );

    // {4}{W}: a 3/4 white Dragon with flying, and still a land.
    let mut engine = Duel::new(1062, forest())
        // Five other lands, so the Cave itself arrives tapped — which the
        // animation does not care about, because "{4}{W}:" charges mana and
        // not a tap. The Plains is the white half of that price: the Cave's
        // own mana ability is the card's only other white source and a tapped
        // land cannot pay.
        .battlefield(0, &[cave, plains(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let id = on_battlefield(&engine, p0, cave).expect("the Cave is on the battlefield");

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
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(object, index)| *object == id && *index == 1)
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
            .object(id)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let types = engine
        .state()
        .object(id)
        .expect("the Cave exists")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::CREATURE),
        "it never became a Dragon"
    );
    assert!(types.contains(TypeSet::LAND), "\"It's still a land\"");
    assert_eq!(pt(&engine, id), (3, 4), "a 3/4 Dragon");
}

/// Bazaar of Baghdad: "{T}: Draw two cards, then discard three cards."
/// Activating Bazaar of Baghdad draws two cards from the library and then requires
/// discarding three cards, leaving the hand one card smaller and the land tapped.
#[test]
fn bazaar_of_baghdad_draws_two_and_discards_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[bazaar_of_baghdad()])
        // Three to discard needs three to discard from: the kit deals no
        // opening hand, so without this the engine clamps the choice to the
        // two cards Bazaar itself drew and the test measures the clamp.
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bazaar = on_battlefield(&engine, p0, bazaar_of_baghdad()).expect("Bazaar deployed");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    activate(&mut engine, p0, bazaar_of_baghdad(), 0);

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
        panic!("expected discard choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (3, 3));
    assert_eq!(prompt, ChoicePrompt::Generic);

    let discards: Vec<ObjectId> = options.into_iter().take(3).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: discards })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 2, "drew two cards");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 3,
        "discarded three cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "net hand size decreased by one"
    );
    assert!(is_tapped(&engine, bazaar));
}

/// City of Ass: "This land enters tapped." / "{T}: Add one and one-half mana of any one color."
/// Under `Coverage::Partial`, the fractional mana ability is unsupported and omitted from the card.
/// When played from hand, City of Ass enters tapped and offers no mana abilities.
#[test]
fn city_of_ass_enters_tapped_and_omits_unsupported_mana_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest()).hand(0, &[city_of_ass()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ass = play_land(&mut engine, p0, city_of_ass());
    assert!(is_tapped(&engine, ass), "City of Ass enters tapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&ass),
        "no intrinsic mana ability"
    );
    assert!(
        !legal.abilities.iter().any(|(s, _)| *s == ass),
        "fractional mana ability is not implemented"
    );
}

/// City of Traitors: "When you play another land, sacrifice this land." / "{T}: Add {C}{C}."
/// City of Traitors produces two colorless mana from its printed mana ability.
/// When another land enters under its controller's control, its trigger fires and sacrifices it.
#[test]
fn city_of_traitors_taps_for_two_colorless_and_sacrifices_on_another_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(82, forest())
        .battlefield(0, &[city_of_traitors()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let city = on_battlefield(&engine, p0, city_of_traitors()).expect("City of Traitors deployed");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );

    // Ability 0 is the trigger; ability 1 is the mana ability.
    activate(&mut engine, p0, city_of_traitors(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "City of Traitors adds {{C}}{{C}}"
    );
    assert!(is_tapped(&engine, city));

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, city_of_traitors()).is_none(),
        "City of Traitors was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, city_of_traitors()).is_some(),
        "City of Traitors is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "the played Forest remains on battlefield"
    );
}

/// Desolate Lighthouse: "{1}{U}{R}, {T}: Draw a card, then discard a card."
/// An Island, a Mountain, and a Forest pay the {1}{U}{R} cost to loot.
/// The controller draws one card, selects one card to discard, and the land remains tapped.
#[test]
fn desolate_lighthouse_loots_with_mana_and_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(93, forest())
        .battlefield(0, &[desolate_lighthouse(), island(), mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lighthouse =
        on_battlefield(&engine, p0, desolate_lighthouse()).expect("Lighthouse deployed");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    tap_mana_except(&mut engine, p0, lighthouse);
    activate(&mut engine, p0, desolate_lighthouse(), 1);

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
        panic!("expected discard prompt, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::Generic);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "drew one and discarded one: net hand size unchanged"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 1,
        "discarded card is in graveyard"
    );
    assert!(is_tapped(&engine, lighthouse));
}

/// Elephant Graveyard: "{T}: Add {C}." / "{T}: Regenerate target Elephant."
/// Under `Coverage::Partial`, regeneration shields are unsupported and the second ability is omitted.
/// When played from hand, Elephant Graveyard offers only its colorless mana ability and taps for {C}.
#[test]
fn elephant_graveyard_taps_for_colorless_and_omits_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(34, forest())
        .hand(0, &[elephant_graveyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let eg = play_land(&mut engine, p0, elephant_graveyard());
    assert!(!is_tapped(&engine, eg));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(eg, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == eg && *i == 1),
        "unsupported regenerate ability is omitted"
    );

    activate(&mut engine, p0, elephant_graveyard(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, eg));
}

/// Geier Reach Sanitarium: "{2}, {T}: Each player draws a card, then discards a card."
/// Two Forests pay {2} while Geier Reach Sanitarium taps to activate its symmetrical looting ability.
/// Both players draw a card and are each sequentially prompted to discard a card to their graveyards.
#[test]
fn geier_reach_sanitarium_each_player_draws_and_discards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[geier_reach_sanitarium(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanitarium =
        on_battlefield(&engine, p0, geier_reach_sanitarium()).expect("Sanitarium deployed");
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, sanitarium);
    activate(&mut engine, p0, geier_reach_sanitarium(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 discard prompt, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        player: p1_player,
        options: p1_options,
        min: p1_min,
        max: p1_max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 discard prompt, got {:?}", engine.pending())
    };
    assert_eq!(p1_player, p1);
    assert_eq!((p1_min, p1_max), (1, 1));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        p0_gy_before + 1,
        "p0 discarded one card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1,
        "p1 discarded one card"
    );
    assert!(is_tapped(&engine, sanitarium));
}

/// Oboro, Palace in the Clouds: "{T}: Add {U}." and "{1}: Return Oboro to its owner's hand."
/// Oboro taps for blue mana, which remains in the pool to pay for its own return ability.
/// The ability resolves, returning the tapped land back to its owner's hand.
// Was red: the ability resolved and Oboro did not move, because
// `Effect::ReturnToHand` threw its own `TargetSpec` away and read the targets
// chosen at activation — of which there are none, since naming your own
// source chooses nothing. `resolve::zones::spec_object` reads the spec; the
// rule's own tests are in `engine::this_object_tests` (#147).
#[test]
fn oboro_palace_in_the_clouds_taps_for_blue_and_returns_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(14, forest())
        .battlefield(0, &[oboro_palace_in_the_clouds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let oboro = on_battlefield(&engine, p0, oboro_palace_in_the_clouds()).expect("Oboro deployed");
    activate(&mut engine, p0, oboro_palace_in_the_clouds(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "Oboro tapped for {{U}}"
    );
    assert!(is_tapped(&engine, oboro));

    activate(&mut engine, p0, oboro_palace_in_the_clouds(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, oboro_palace_in_the_clouds()).is_none(),
        "Oboro left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, oboro_palace_in_the_clouds()).is_some(),
        "Oboro returned to owner's hand"
    );
}

/// Scavenger Grounds: "{2}, {T}, Sacrifice a Desert: Exile all graveyards."
/// Both players have cards in their graveyards, and two Forests pay the generic cost.
/// Scavenger Grounds sacrifices itself as the chosen Desert, exiling all graveyards.
#[test]
fn scavenger_grounds_sacrifices_a_desert_to_exile_all_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(63, forest())
        .battlefield(0, &[scavenger_grounds(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 2);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2
    );

    let sg = on_battlefield(&engine, p0, scavenger_grounds()).expect("Scavenger Grounds deployed");
    tap_mana_except(&mut engine, p0, sg);

    activate(&mut engine, p0, scavenger_grounds(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&sg),
        "Scavenger Grounds is a Desert and can be sacrificed"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![sg] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "p0 graveyard is completely exiled"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "p1 graveyard is completely exiled"
    );
    assert!(on_battlefield(&engine, p0, scavenger_grounds()).is_none());
}

/// Shivan Gorge: "{T}: Add {C}." / "{2}{R}, {T}: Shivan Gorge deals 1 damage to each opponent."
/// Under `Coverage::Partial`, dealing damage to each opponent is unsupported and omitted.
/// Shivan Gorge is played as a legendary land and taps for {C}, offering no damage activation.
#[test]
fn shivan_gorge_taps_for_colorless_and_omits_unsupported_damage_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(28, forest())
        .hand(0, &[shivan_gorge()])
        .battlefield(0, &[mountain(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gorge = play_land(&mut engine, p0, shivan_gorge());
    assert!(!is_tapped(&engine, gorge));

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, gorge);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(gorge, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == gorge && *i == 1),
        "unsupported damage ability is omitted"
    );

    activate(&mut engine, p0, shivan_gorge(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, gorge));
}

/// Throne of the High City: "{4}, {T}, Sacrifice this land: You become the monarch."
/// Four Forests pay the generic cost to activate Throne of the High City.
/// The land is sacrificed as a cost, and upon resolution, its controller becomes the monarch.
#[test]
fn throne_of_the_high_city_sacrifices_to_make_controller_monarch() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(66, forest())
        .battlefield(
            0,
            &[
                throne_of_the_high_city(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().monarch, None);
    let throne = on_battlefield(&engine, p0, throne_of_the_high_city()).expect("Throne deployed");
    tap_mana_except(&mut engine, p0, throne);

    activate(&mut engine, p0, throne_of_the_high_city(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().monarch, Some(p0), "p0 became the monarch");
    assert!(on_battlefield(&engine, p0, throne_of_the_high_city()).is_none());
    assert!(in_graveyard(&engine, p0, throne_of_the_high_city()).is_some());
}

/// Treasure Vault: "{T}: Add {C}." and "{X}{X}, {T}, Sacrifice this land: Create X Treasure tokens."
/// Treasure Vault is played as an artifact land, entering untapped with both types.
/// Its mana ability is offered and tapped to produce {C}, and its activated sacrifice ability is offered.
#[test]
fn treasure_vault_enters_as_artifact_land_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(88, forest()).hand(0, &[treasure_vault()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = play_land(&mut engine, p0, treasure_vault());
    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, vault));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "ability 0 is the printed colorless mana ability"
    );
    assert!(
        legal.abilities.contains(&(vault, 1)),
        "ability 1 is the sacrifice ability"
    );

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
    activate(&mut engine, p0, treasure_vault(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, vault));
}

/// Watermarket: "{T}: Add {C}{C}. Spend this mana only to cast spells with watermarks."
/// Under `Coverage::Partial`, watermark spend restrictions are unsupported and the mana is made unrestricted.
/// Activating Watermarket's printed mana ability adds two colorless mana to the pool and taps the land.
#[test]
fn watermarket_taps_for_two_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[watermarket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wm = on_battlefield(&engine, p0, watermarket()).expect("Watermarket deployed");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );

    activate(&mut engine, p0, watermarket(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "Watermarket produces {{C}}{{C}}"
    );
    assert!(is_tapped(&engine, wm));
}

/// Witch's Clinic: "{T}: Add {C}." / "{2}, {T}: Target commander gains lifelink until end of turn."
/// Under `Coverage::Partial`, targeting a commander is unsupported and the lifelink ability is omitted.
/// Witch's Clinic is played as a land and taps for colorless mana, offering no lifelink activation.
#[test]
fn witch_s_clinic_taps_for_colorless_and_omits_unsupported_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(99, forest())
        .hand(0, &[witch_s_clinic()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clinic = play_land(&mut engine, p0, witch_s_clinic());
    assert!(!is_tapped(&engine, clinic));

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, clinic);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(clinic, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == clinic && *i == 1),
        "unsupported commander lifelink ability is omitted"
    );

    activate(&mut engine, p0, witch_s_clinic(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, clinic));
}

/// Yavimaya Hollow: "{T}: Add {C}." / "{G}, {T}: Regenerate target creature."
/// Under `Coverage::Partial`, regeneration shields are unsupported and the second ability is omitted.
/// When played from hand, Yavimaya Hollow offers only its colorless mana ability and taps for {C}.
#[test]
fn yavimaya_hollow_taps_for_colorless_and_omits_unsupported_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(45, forest())
        .hand(0, &[yavimaya_hollow()])
        .battlefield(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, yavimaya_hollow());
    assert!(!is_tapped(&engine, hollow));

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, hollow);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hollow, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == hollow && *i == 1),
        "unsupported regenerate ability is omitted"
    );

    activate(&mut engine, p0, yavimaya_hollow(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, hollow));
}

/// Access Tunnel: "{T}: Add {C}." / "{3}, {T}: Target creature with power 3 or less can't be blocked this turn."
/// Under `Coverage::Partial`, the power-restricted unblockable ability is unsupported and omitted.
/// The land taps for its mana ability adding {C} and offers no second ability.
#[test]
fn access_tunnel_taps_for_colorless_and_omits_unblockable_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[access_tunnel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = on_battlefield(&engine, p0, access_tunnel()).expect("Access Tunnel deployed");

    activate(&mut engine, p0, access_tunnel(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tunnel));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == tunnel && *ai == 1),
        "no second ability is offered"
    );
}

/// Alchemist's Refuge: "{T}: Add {C}." / "{G}{U}, {T}: You may cast spells this turn as though they had flash."
/// Under `Coverage::Partial`, the global flash-granting permission is unsupported and omitted.
/// The land taps to add {C} to the mana pool and offers no second ability.
#[test]
fn alchemist_s_refuge_taps_for_colorless_and_omits_flash_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .battlefield(0, &[alchemist_s_refuge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let refuge = on_battlefield(&engine, p0, alchemist_s_refuge()).expect("Refuge deployed");

    activate(&mut engine, p0, alchemist_s_refuge(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, refuge));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == refuge && *ai == 1),
        "no second ability is offered"
    );
}

/// Cinder Marsh: "{T}: Add {C}." / "{T}: Add {B} or {R}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Cinder Marsh remains tapped.
#[test]
fn cinder_marsh_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(108, forest())
        .battlefield(0, &[cinder_marsh(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let marsh = on_battlefield(&engine, p0, cinder_marsh()).expect("Cinder Marsh deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, cinder_marsh(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert!(is_tapped(&engine, marsh));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, marsh),
        "Cinder Marsh stays tapped during your next untap step"
    );
}

/// Cloudcrest Lake: "{T}: Add {C}." / "{T}: Add {W} or {U}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Cloudcrest Lake remains tapped.
#[test]
fn cloudcrest_lake_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(109, forest())
        .battlefield(0, &[cloudcrest_lake(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lake = on_battlefield(&engine, p0, cloudcrest_lake()).expect("Cloudcrest Lake deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, cloudcrest_lake(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert!(is_tapped(&engine, lake));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, lake),
        "Cloudcrest Lake stays tapped during your next untap step"
    );
}

/// Drownyard Temple: "{T}: Add {C}." / "{3}: Return this card from your graveyard to the battlefield tapped."
/// Under `Coverage::Partial`, activating from the graveyard to return tapped is unsupported and omitted.
/// The land taps on the battlefield to add {C} to the mana pool.
#[test]
fn drownyard_temple_taps_for_colorless_and_omits_graveyard_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest())
        .battlefield(0, &[drownyard_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, drownyard_temple()).expect("Temple deployed");

    activate(&mut engine, p0, drownyard_temple(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, temple));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == temple && *ai == 1),
        "no second ability is offered"
    );
}

/// Forgotten Monument: "{T}: Add {C}." / "Other Caves you control have '{T}, Pay 1 life: Add one mana of any color.'"
/// Under `Coverage::Partial`, the ability-granting static to other Caves is unsupported and omitted.
/// The land taps for its intrinsic printed ability to add {C} to the mana pool.
#[test]
fn forgotten_monument_taps_for_colorless_mana_and_omits_ability_grant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[forgotten_monument()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monument = on_battlefield(&engine, p0, forgotten_monument()).expect("Monument deployed");

    activate(&mut engine, p0, forgotten_monument(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, monument));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == monument && *ai == 1),
        "no second ability is offered on Forgotten Monument"
    );
}

/// Gavony Township: "{T}: Add {C}." / "{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."
/// Paid with four basic lands, the activated ability resolves to bolster all controlled creatures.
/// Llanowar Elves receives a +1/+1 counter, increasing its power and toughness to 2/2.
#[test]
fn gavony_township_puts_plus_one_plus_one_counter_on_each_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(
            0,
            &[
                gavony_township(),
                forest(),
                forest(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let township = on_battlefield(&engine, p0, gavony_township()).expect("Township deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_mana_except(&mut engine, p0, township);
    activate(&mut engine, p0, gavony_township(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, elves), (2, 2));
    assert!(is_tapped(&engine, township));
}

/// Glimmerpost: "When this land enters, you gain 1 life for each Locus on the battlefield." / "{T}: Add {C}."
/// When a second Glimmerpost enters a battlefield that already controls one Locus, its trigger counts both.
/// Upon resolution, the player gains 2 life and the land can be tapped for {C}.
#[test]
fn glimmerpost_enters_gaining_life_per_locus_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[glimmerpost()])
        .hand(0, &[glimmerpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let post = play_land(&mut engine, p0, glimmerpost());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "two Loci on the battlefield gain 2 life"
    );

    // Addressed by object, not by card: two Glimmerposts are on the
    // battlefield and `activate` would take whichever the legal set offers
    // first, which is the one that was already there.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: post,
                ability_index: 0,
            },
        )
        .expect("the mana ability activates");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, post));
}

/// Hammerheim: "{T}: Add {R}." / "{T}: Target creature loses all landwalk abilities until end of turn."
/// Under `Coverage::Partial`, the landwalk-removal ability is unsupported and omitted.
/// The legendary land taps to add {R} to the mana pool and offers no second ability.
#[test]
fn hammerheim_taps_for_red_mana_and_omits_landwalk_removal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[hammerheim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hammer = on_battlefield(&engine, p0, hammerheim()).expect("Hammerheim deployed");

    activate(&mut engine, p0, hammerheim(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, hammer));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == hammer && *ai == 1),
        "no second ability is offered"
    );
}

/// Labyrinth of Skophos: "{T}: Add {C}." / "{4}, {T}: Remove target attacking or blocking creature from combat."
/// Under `Coverage::Partial`, the ability to remove a creature from combat is unsupported and omitted.
/// The land taps for its mana ability adding {C} to the mana pool.
#[test]
fn labyrinth_of_skophos_taps_for_colorless_and_omits_combat_removal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[labyrinth_of_skophos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, labyrinth_of_skophos()).expect("Labyrinth deployed");

    activate(&mut engine, p0, labyrinth_of_skophos(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lab));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == lab && *ai == 1),
        "no second ability is offered"
    );
}

/// Lantern-Lit Graveyard: "{T}: Add {C}." / "{T}: Add {B} or {R}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Lantern-Lit Graveyard remains tapped.
#[test]
fn lantern_lit_graveyard_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[lantern_lit_graveyard(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let graveyard =
        on_battlefield(&engine, p0, lantern_lit_graveyard()).expect("Graveyard deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, lantern_lit_graveyard(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert!(is_tapped(&engine, graveyard));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, graveyard),
        "Lantern-Lit Graveyard stays tapped during your next untap step"
    );
}

/// Library of Alexandria: "{T}: Add {C}." / "{T}: Draw a card. Activate only if you have exactly seven cards in hand."
/// Under `Coverage::Partial`, the conditional draw ability is unsupported and omitted.
/// The land taps to add {C} to the mana pool and offers no second ability.
#[test]
fn library_of_alexandria_taps_for_colorless_and_omits_draw_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[library_of_alexandria()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib = on_battlefield(&engine, p0, library_of_alexandria()).expect("Library deployed");

    activate(&mut engine, p0, library_of_alexandria(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lib));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == lib && *ai == 1),
        "draw ability is not offered"
    );
}

/// Mogg Hollows: "{T}: Add {C}." / "{T}: Add {R} or {G}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Mogg Hollows remains tapped.
#[test]
fn mogg_hollows_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[mogg_hollows(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollows = on_battlefield(&engine, p0, mogg_hollows()).expect("Mogg Hollows deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, mogg_hollows(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, hollows));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, hollows),
        "Mogg Hollows stays tapped during your next untap step"
    );
}

/// Novijen, Heart of Progress: "{T}: Add {C}." / "{G}{U}, {T}: Put a +1/+1 counter on each creature that entered this turn."
/// Under `Coverage::Partial`, filtering creatures by entry time is unsupported and omitted.
/// The land taps to add {C} to the mana pool and offers no second ability.
#[test]
fn novijen_heart_of_progress_taps_for_colorless_and_omits_counter_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[novijen_heart_of_progress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let novijen =
        on_battlefield(&engine, p0, novijen_heart_of_progress()).expect("Novijen deployed");

    activate(&mut engine, p0, novijen_heart_of_progress(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, novijen));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == novijen && *ai == 1),
        "no second ability is offered"
    );
}

/// Pillar of the Paruns: "{T}: Add one mana of any color. Spend this mana only to cast a multicolored spell."
/// Activating the land produces mana marked with a multicolored spell restriction.
/// Alongside a Swamp, the restricted blue mana successfully casts Baleful Strix.
#[test]
fn pillar_of_the_paruns_adds_restricted_mana_for_multicolored_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[pillar_of_the_paruns(), swamp()])
        .hand(0, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, pillar_of_the_paruns(), 0);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);

    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let strix = in_hand(&engine, p0, baleful_strix()).expect("Strix in hand");
    assert!(
        legal.castable.contains(&strix),
        "Baleful Strix is castable with restricted mana"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: strix })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, baleful_strix()).is_some(),
        "Baleful Strix resolved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "restricted mana was spent"
    );
}

/// Pinecrest Ridge: "{T}: Add {C}." / "{T}: Add {R} or {G}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Pinecrest Ridge remains tapped.
#[test]
fn pinecrest_ridge_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[pinecrest_ridge(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ridge = on_battlefield(&engine, p0, pinecrest_ridge()).expect("Pinecrest Ridge deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, pinecrest_ridge(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, ridge));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, ridge),
        "Pinecrest Ridge stays tapped during your next untap step"
    );
}

/// Rix Maadi, Dungeon Palace: "{T}: Add {C}." / "{1}{B}{R}, {T}: Each player discards a card. Activate only as a sorcery."
/// Paid with three lands, the sorcery-speed activation triggers symmetrical discard.
/// Both players are prompted in order to choose a card to discard to their graveyards.
#[test]
fn rix_maadi_dungeon_palace_forces_each_player_to_discard_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(129, forest())
        .battlefield(
            0,
            &[rix_maadi_dungeon_palace(), swamp(), mountain(), forest()],
        )
        // The kit deals no opening hand, and a player with nothing to discard
        // is not asked. Two cards each, so the prompt is a real choice.
        .hand(0, &[forest(), forest()])
        .hand(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace =
        on_battlefield(&engine, p0, rix_maadi_dungeon_palace()).expect("Rix Maadi deployed");
    let p0_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, palace);
    activate(&mut engine, p0, rix_maadi_dungeon_palace(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 discard prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        player: p1_player,
        options: p1_options,
        min: p1_min,
        max: p1_max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 discard prompt, got {:?}", engine.pending());
    };
    assert_eq!(p1_player, p1);
    assert_eq!((p1_min, p1_max), (1, 1));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        p0_gy_before + 1,
        "p0 discarded 1 card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 1,
        "p1 discarded 1 card"
    );
    assert!(is_tapped(&engine, palace));
}

/// Rootwater Depths: "{T}: Add {C}." / "{T}: Add {U} or {B}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Rootwater Depths remains tapped.
#[test]
fn rootwater_depths_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[rootwater_depths(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let depths =
        on_battlefield(&engine, p0, rootwater_depths()).expect("Rootwater Depths deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, rootwater_depths(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, depths));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, depths),
        "Rootwater Depths stays tapped during your next untap step"
    );
}

/// Stensia Bloodhall: "{T}: Add {C}." / "{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker."
/// Under `Coverage::Partial`, targeting players or planeswalkers in union is unsupported and omitted.
/// The land taps for its mana ability adding {C} to the mana pool.
#[test]
fn stensia_bloodhall_taps_for_colorless_and_omits_damage_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[stensia_bloodhall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, stensia_bloodhall()).expect("Bloodhall deployed");

    activate(&mut engine, p0, stensia_bloodhall(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hall));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == hall && *ai == 1),
        "no second ability is offered"
    );
}

/// Sunscorched Desert: "When this land enters, it deals 1 damage to target player or planeswalker." / "{T}: Add {C}."
/// Under `Coverage::Partial`, the enters-the-battlefield trigger is omitted due to target specification constraints.
/// The land enters without triggering damage and taps to add {C} to the mana pool.
#[test]
fn sunscorched_desert_enters_without_trigger_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .hand(0, &[sunscorched_desert()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_life_before = engine.state().players[1].life;
    let desert = play_land(&mut engine, p0, sunscorched_desert());

    assert!(stack_is_empty(&engine), "no enters trigger on the stack");
    assert_eq!(
        engine.state().players[1].life,
        p1_life_before,
        "opponent life unchanged"
    );

    activate(&mut engine, p0, sunscorched_desert(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, desert));
}

/// Thalakos Lowlands: "{T}: Add {C}." / "{T}: Add {W} or {U}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Thalakos Lowlands remains tapped.
#[test]
fn thalakos_lowlands_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[thalakos_lowlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lowlands =
        on_battlefield(&engine, p0, thalakos_lowlands()).expect("Thalakos Lowlands deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, thalakos_lowlands(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert!(is_tapped(&engine, lowlands));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, lowlands),
        "Thalakos Lowlands stays tapped during your next untap step"
    );
}

/// Tomb of the Spirit Dragon: "{T}: Add {C}." / "{2}, {T}: You gain 1 life for each colorless creature you control."
/// With two colorless artifact creatures (Myr Retrievers) under control, the activated ability is paid with two Forests.
/// Upon resolution, the player gains two life, advancing from 20 to 22.
#[test]
fn tomb_of_the_spirit_dragon_gains_life_per_colorless_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .battlefield(
            0,
            &[
                tomb_of_the_spirit_dragon(),
                forest(),
                forest(),
                myr_retriever(),
                myr_retriever(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tomb = on_battlefield(&engine, p0, tomb_of_the_spirit_dragon()).expect("Tomb deployed");
    let life_before = engine.state().players[0].life;

    tap_mana_except(&mut engine, p0, tomb);
    activate(&mut engine, p0, tomb_of_the_spirit_dragon(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "gained 2 life for 2 colorless creatures"
    );
    assert!(is_tapped(&engine, tomb));
}

/// Tranquil Garden: "{T}: Add {C}." / "{T}: Add {G} or {W}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Tranquil Garden remains tapped.
#[test]
fn tranquil_garden_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(115, forest())
        .battlefield(0, &[tranquil_garden(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let garden = on_battlefield(&engine, p0, tranquil_garden()).expect("Tranquil Garden deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, tranquil_garden(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, garden));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, garden),
        "Tranquil Garden stays tapped during your next untap step"
    );
}

/// Trenchpost: "{T}: Add {C}." / "{3}, {T}: Target player mills a card for each Locus you control."
/// With Trenchpost and a second Locus under control, the activated ability is pointed at the opponent.
/// Upon resolution, the opponent mills two cards into their graveyard.
#[test]
fn trenchpost_mills_target_player_per_locus_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(107, forest())
        .battlefield(
            0,
            &[trenchpost(), glimmerpost(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let trench = on_battlefield(&engine, p0, trenchpost()).expect("Trenchpost deployed");
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_mana_except(&mut engine, p0, trench);
    activate(&mut engine, p0, trenchpost(), 1);

    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(player_options.contains(&p1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 2,
        "opponent milled 2 cards for 2 Loci you control"
    );
    assert!(is_tapped(&engine, trench));
}

/// Underdome: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to pay Un-costs."
/// Under `Coverage::Partial`, the Un-cost restricted mana ability is unsupported and omitted.
/// The land taps to produce {C} and offers no additional activated ability.
#[test]
fn underdome_taps_for_colorless_and_omits_un_costs_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[underdome()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, underdome()).expect("Underdome deployed");

    activate(&mut engine, p0, underdome(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == land && *ai == 1),
        "no second ability is offered"
    );
}

/// Urborg: "{T}: Add {B}." / "{T}: Target creature loses first strike or swampwalk until end of turn."
/// Under `Coverage::Partial`, the modal keyword loss ability is unsupported and omitted.
/// The legendary land taps to add {B} to the mana pool and offers no second ability.
#[test]
fn urborg_taps_for_black_mana_and_omits_keyword_loss() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(132, forest()).battlefield(0, &[urborg()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let u = on_battlefield(&engine, p0, urborg()).expect("Urborg deployed");

    activate(&mut engine, p0, urborg(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, u));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == u && *ai == 1),
        "no second ability is offered"
    );
}

/// Urza's Power Plant: "{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead."
/// Under `Coverage::Partial`, the multi-permanent Tron replacement condition is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn urza_s_power_plant_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[urza_s_power_plant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plant = on_battlefield(&engine, p0, urza_s_power_plant()).expect("Power Plant deployed");

    activate(&mut engine, p0, urza_s_power_plant(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, plant));
}

/// Vec Townships: "{T}: Add {C}." / "{T}: Add {G} or {W}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Vec Townships remains tapped.
#[test]
fn vec_townships_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[vec_townships(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let townships = on_battlefield(&engine, p0, vec_townships()).expect("Vec Townships deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, vec_townships(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, townships));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, townships),
        "Vec Townships stays tapped during your next untap step"
    );
}

/// Waterveil Cavern: "{T}: Add {C}." / "{T}: Add {U} or {B}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Waterveil Cavern remains tapped.
#[test]
fn waterveil_cavern_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[waterveil_cavern(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cavern =
        on_battlefield(&engine, p0, waterveil_cavern()).expect("Waterveil Cavern deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, waterveil_cavern(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, cavern));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, cavern),
        "Waterveil Cavern stays tapped during your next untap step"
    );
}

/// Winding Canyons: "{T}: Add {C}." / "{2}, {T}: You may cast creature spells this turn as though they had flash."
/// Under `Coverage::Partial`, granting flash to creature spells in hand is unsupported and omitted.
/// The land taps to produce {C} and offers no second ability.
#[test]
fn winding_canyons_taps_for_colorless_and_omits_creature_flash_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[winding_canyons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let canyons = on_battlefield(&engine, p0, winding_canyons()).expect("Canyons deployed");

    activate(&mut engine, p0, winding_canyons(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, canyons));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == canyons && *ai == 1),
        "no second ability is offered"
    );
}

/// Barkchannel Pathway // Tidechannel Pathway: Modal double-faced land.
/// Front face taps for {G}; back face Tidechannel Pathway taps for {U}.
/// Playing the card from hand and choosing face 1 puts Tidechannel Pathway onto the battlefield.
/// The land enters untapped and taps to add {U} to the mana pool.
#[test]
fn barkchannel_pathway_back_face_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .hand(0, &[barkchannel_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, barkchannel_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, barkchannel_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, barkchannel_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Blighted Fen: "{T}: Add {C}." / "{4}{B}, {T}, Sacrifice this land: Target opponent sacrifices a creature of their choice."
/// Paid with five Swamps, the sacrifice ability targets the opponent.
/// On resolution, the opponent chooses and sacrifices Llanowar Elves while Blighted Fen is in its owner's graveyard.
#[test]
fn blighted_fen_forces_opponent_to_sacrifice_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(132, forest())
        .battlefield(
            0,
            &[blighted_fen(), swamp(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fen = on_battlefield(&engine, p0, blighted_fen()).expect("Fen deployed");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("Elves deployed");

    tap_mana_except(&mut engine, p0, fen);
    activate(&mut engine, p0, blighted_fen(), 1);

    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(player_options.contains(&p1));

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
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(chooser, p1);
    assert!(options.contains(&elves));

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, blighted_fen()).is_none());
    assert!(in_graveyard(&engine, p0, blighted_fen()).is_some());
}

/// Blighted Steppe: "{T}: Add {C}." / "{3}{W}, {T}, Sacrifice this land: You gain 2 life for each creature you control."
/// Under `Coverage::Partial`, the scaling life-gain sacrifice ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn blighted_steppe_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[blighted_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let steppe = on_battlefield(&engine, p0, blighted_steppe()).expect("Steppe deployed");
    activate(&mut engine, p0, blighted_steppe(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, steppe));
}

/// Blightstep Pathway // Searstep Pathway: Modal double-faced land.
/// Front face taps for {B}; back face Searstep Pathway taps for {R}.
/// Playing the card from hand and choosing face 1 puts Searstep Pathway onto the battlefield.
/// The land enters untapped and taps to add {R} to the mana pool.
#[test]
fn blightstep_pathway_back_face_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .hand(0, &[blightstep_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, blightstep_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, blightstep_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, blightstep_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Bonders' Enclave: "{T}: Add {C}." / "{3}, {T}: Draw a card. Activate only if you control a creature with power 4 or greater."
/// Under `Coverage::Partial`, the conditional card draw ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn bonders_enclave_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[bonders_enclave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enclave = on_battlefield(&engine, p0, bonders_enclave()).expect("Enclave deployed");
    activate(&mut engine, p0, bonders_enclave(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, enclave));
}

/// Branchloft Pathway // Boulderloft Pathway: Modal double-faced land.
/// Front face taps for {G}; back face Boulderloft Pathway taps for {W}.
/// Playing the card from hand and choosing face 1 puts Boulderloft Pathway onto the battlefield.
/// The land enters untapped and taps to add {W} to the mana pool.
#[test]
fn branchloft_pathway_back_face_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .hand(0, &[branchloft_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, branchloft_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, branchloft_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, branchloft_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Command Beacon: "{T}: Add {C}." / "{T}, Sacrifice this land: Put your commander into your hand from the command zone."
/// Under `Coverage::Partial`, the command-zone return ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn command_beacon_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[command_beacon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beacon = on_battlefield(&engine, p0, command_beacon()).expect("Beacon deployed");
    activate(&mut engine, p0, command_beacon(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, beacon));
}

/// Cragcrown Pathway // Timbercrown Pathway: Modal double-faced land.
/// Front face taps for {R}; back face Timbercrown Pathway taps for {G}.
/// Playing the card from hand and choosing face 1 puts Timbercrown Pathway onto the battlefield.
/// The land enters untapped and taps to add {G} to the mana pool.
#[test]
fn cragcrown_pathway_back_face_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .hand(0, &[cragcrown_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, cragcrown_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, cragcrown_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cragcrown_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Crypt of Agadeem: "This land enters tapped." / "{T}: Add {B}." / "{2}, {T}: Add {B} for each black creature card in your graveyard."
/// With two black creature cards in the graveyard and {2} mana available from two Swamps, ability 1 is activated.
/// The ability counts both black creature cards and adds {B}{B} to the mana pool.
#[test]
fn crypt_of_agadeem_adds_black_mana_per_black_creature_in_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, sheoldred_the_apocalypse())
        .battlefield(0, &[crypt_of_agadeem(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);
    let crypt = on_battlefield(&engine, p0, crypt_of_agadeem()).expect("Crypt deployed");

    tap_mana_except(&mut engine, p0, crypt);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2
    );

    activate(&mut engine, p0, crypt_of_agadeem(), 1);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "spent 2 black mana to activate, gained 2 black mana from 2 black creature cards"
    );
    assert!(is_tapped(&engine, crypt));
}

/// Darkbore Pathway // Slitherbore Pathway: Modal double-faced land.
/// Front face taps for {B}; back face Slitherbore Pathway taps for {G}.
/// Playing the card from hand and choosing face 1 puts Slitherbore Pathway onto the battlefield.
/// The land enters untapped and taps to add {G} to the mana pool.
#[test]
fn darkbore_pathway_back_face_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .hand(0, &[darkbore_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, darkbore_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, darkbore_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, darkbore_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Drannith Ruins: "{T}: Add {C}." / "{2}, {T}: Put two +1/+1 counters on target non-Human creature that entered this turn."
/// Under `Coverage::Partial`, the entering-this-turn condition is omitted and any non-Human creature may be targeted.
/// Paid with two Forests, activating ability 1 targets Llanowar Elves and places two +1/+1 counters on it.
#[test]
fn drannith_ruins_places_counters_on_non_human_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest())
        .battlefield(0, &[drannith_ruins(), forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ruins = on_battlefield(&engine, p0, drannith_ruins()).expect("Ruins deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_mana_except(&mut engine, p0, ruins);
    activate(&mut engine, p0, drannith_ruins(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, elves), (3, 3));
    assert!(is_tapped(&engine, ruins));
}

/// Dread Statuary: "{T}: Add {C}." / "{4}: This land becomes a 4/2 Golem artifact creature until end of turn. It's still a land."
/// Paid with four Forests, activating ability 1 animates the land without tapping it.
/// Upon resolution, the permanent has types Land, Creature, and Artifact, with 4/2 base P/T.
#[test]
fn dread_statuary_animates_into_golem_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(
            0,
            &[dread_statuary(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let statuary = on_battlefield(&engine, p0, dread_statuary()).expect("Statuary deployed");
    assert!(!types(&engine, statuary).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, statuary);
    activate(&mut engine, p0, dread_statuary(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, statuary);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::ARTIFACT));
    assert_eq!(pt(&engine, statuary), (4, 2));
    assert!(!is_tapped(&engine, statuary));
}

/// Eiganjo Castle: "{T}: Add {W}." / "{W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn."
/// Under `Coverage::Partial`, the activated damage-prevention ability is omitted.
/// The legendary land taps for its base mana ability to add {W} to the mana pool.
#[test]
fn eiganjo_castle_taps_for_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(139, forest())
        .battlefield(0, &[eiganjo_castle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = on_battlefield(&engine, p0, eiganjo_castle()).expect("Castle deployed");
    activate(&mut engine, p0, eiganjo_castle(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, castle));
}

/// Emergence Zone: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: You may cast spells this turn as though they had flash."
/// Under `Coverage::Partial`, the timing grant applies to sorceries rather than all spell types.
/// Activating ability 1 sacrifices Emergence Zone, paying {1} from a Forest and sending the land to the graveyard.
#[test]
fn emergence_zone_sacrifices_to_grant_flash() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[emergence_zone(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let zone = on_battlefield(&engine, p0, emergence_zone()).expect("Zone deployed");
    tap_mana_except(&mut engine, p0, zone);

    activate(&mut engine, p0, emergence_zone(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, emergence_zone()).is_none());
    assert!(in_graveyard(&engine, p0, emergence_zone()).is_some());
}

/// Ghost Town: "{T}: Add {C}." / "{0}: Return this land to its owner's hand. Activate only if it's not your turn."
/// Under `Coverage::Partial`, the not-your-turn timing restriction is omitted.
/// Activating ability 1 for {0} returns Ghost Town from the battlefield to its owner's hand.
#[test]
fn ghost_town_returns_to_hand_on_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[ghost_town()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, ghost_town()).is_some());
    assert!(in_hand(&engine, p0, ghost_town()).is_none());

    activate(&mut engine, p0, ghost_town(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, ghost_town()).is_none());
    assert!(in_hand(&engine, p0, ghost_town()).is_some());
}

/// Hall of Heliod's Generosity: "{T}: Add {C}." / "{1}{W}, {T}: Put target enchantment card from your graveyard on top of your library."
/// With an enchantment card in the graveyard and mana from two Plains, ability 1 targets the enchantment.
/// Upon resolution, the enchantment card is moved from the graveyard to the top of the library.
#[test]
fn hall_of_heliod_s_generosity_puts_enchantment_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, luminarch_ascension())
        .battlefield(0, &[hall_of_heliod_s_generosity(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let hall = on_battlefield(&engine, p0, hall_of_heliod_s_generosity()).expect("Hall deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let target_ench = gy[0];

    tap_mana_except(&mut engine, p0, hall);
    activate(&mut engine, p0, hall_of_heliod_s_generosity(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_ench));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_ench],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0,
        "enchantment left the graveyard"
    );
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.last().copied(),
        Some(target_ench),
        "enchantment on top of library"
    );
    assert!(is_tapped(&engine, hall));
}

/// Kessig Wolf Run: "{T}: Add {C}." / "{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn."
/// With mana from a Forest and a Mountain, the second ability is activated targeting Llanowar Elves with X=0.
/// Upon resolution, Llanowar Elves gains trample until end of turn and the land is tapped.
#[test]
fn kessig_wolf_run_grants_trample_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(
            0,
            &[kessig_wolf_run(), forest(), mountain(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf_run = on_battlefield(&engine, p0, kessig_wolf_run()).expect("Wolf Run deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert!(!keywords(&engine, elves).contains(KeywordSet::TRAMPLE));

    tap_mana_except(&mut engine, p0, wolf_run);
    activate(&mut engine, p0, kessig_wolf_run(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, elves).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, elves), (1, 1));
    assert!(is_tapped(&engine, wolf_run));
}

/// Miren, the Moaning Well: "{T}: Add {C}." / "{3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness."
/// Under `Coverage::Partial`, the toughness-scaled life-gain sacrifice ability is omitted.
/// The legendary land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn miren_the_moaning_well_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(0, &[miren_the_moaning_well()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, miren_the_moaning_well()).expect("Well deployed");
    activate(&mut engine, p0, miren_the_moaning_well(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, well));
}

/// Needleverge Pathway // Pillarverge Pathway: Modal double-faced land.
/// Front face taps for {R}; back face Pillarverge Pathway taps for {W}.
/// Playing the card from hand and choosing face 1 puts Pillarverge Pathway onto the battlefield.
/// The land enters untapped and taps to add {W} to the mana pool.
#[test]
fn needleverge_pathway_back_face_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .hand(0, &[needleverge_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, needleverge_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, needleverge_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, needleverge_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Ominous Cemetery: "{T}: Add {C}." / "{5}, {T}, Exile this land: Target creature's owner shuffles it into their library."
/// Under `Coverage::Partial`, the exile-and-shuffle creature removal ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn ominous_cemetery_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[ominous_cemetery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cemetery = on_battlefield(&engine, p0, ominous_cemetery()).expect("Cemetery deployed");
    activate(&mut engine, p0, ominous_cemetery(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cemetery));
}

/// Petrified Field: "{T}: Add {C}." / "{T}, Sacrifice this land: Return target land card from your graveyard to your hand."
/// With a Forest card in the graveyard, Petrified Field's sacrifice ability targets the Forest.
/// Upon resolution, Petrified Field is in the graveyard and the Forest card is returned to hand.
#[test]
fn petrified_field_returns_land_from_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[petrified_field()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let _field = on_battlefield(&engine, p0, petrified_field()).expect("Field deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let target_land = gy[0];

    activate(&mut engine, p0, petrified_field(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, petrified_field()).is_none());
    assert!(in_graveyard(&engine, p0, petrified_field()).is_some());
    assert!(in_hand(&engine, p0, forest()).is_some());
}

/// Prahv, Spires of Order: "{T}: Add {C}." / "{4}{W}{U}, {T}: Prevent all damage a source of your choice would deal this turn."
/// Under `Coverage::Partial`, the resolution-time damage prevention choice is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn prahv_spires_of_order_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[prahv_spires_of_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prahv = on_battlefield(&engine, p0, prahv_spires_of_order()).expect("Prahv deployed");
    activate(&mut engine, p0, prahv_spires_of_order(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, prahv));
}

/// R&D's Secret Lair: "Play cards as written. Ignore all errata." / "{T}: Add {C}."
/// Under `Coverage::Partial`, the un-set errata-ignoring rule is omitted.
/// The legendary land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn r_d_s_secret_lair_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .battlefield(0, &[r_d_s_secret_lair()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = on_battlefield(&engine, p0, r_d_s_secret_lair()).expect("Lair deployed");
    activate(&mut engine, p0, r_d_s_secret_lair(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lair));
}

/// Rainbow Vale: "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step."
/// Under `Coverage::Partial`, the end-step control change trigger is omitted.
/// Activating the land prompts for a color choice; selecting Green adds {G} to the mana pool.
#[test]
fn rainbow_vale_adds_chosen_color_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[rainbow_vale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = on_battlefield(&engine, p0, rainbow_vale()).expect("Vale deployed");
    activate(&mut engine, p0, rainbow_vale(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, vale));
}

/// Rhystic Cave: "{T}: Choose a color. Add one mana of that color unless any player pays {1}. Activate only as an instant."
/// Under `Coverage::Partial`, the payment prevention condition is omitted.
/// Activating the land prompts for a color choice; selecting Blue adds {U} to the mana pool.
#[test]
fn rhystic_cave_adds_chosen_color_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(0, &[rhystic_cave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cave = on_battlefield(&engine, p0, rhystic_cave()).expect("Cave deployed");
    activate(&mut engine, p0, rhystic_cave(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, cave));
}

/// Riftstone Portal: "{T}: Add {C}." / "As long as this card is in your graveyard, lands you control have '{T}: Add {G} or {W}.'"
/// Under `Coverage::Partial`, the graveyard static ability granting mana abilities to lands is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn riftstone_portal_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .battlefield(0, &[riftstone_portal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let portal = on_battlefield(&engine, p0, riftstone_portal()).expect("Portal deployed");
    activate(&mut engine, p0, riftstone_portal(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, portal));
}

/// Riverglide Pathway // Lavaglide Pathway: Modal double-faced land.
/// Front face taps for {U}; back face Lavaglide Pathway taps for {R}.
/// Playing the card from hand and choosing face 1 puts Lavaglide Pathway onto the battlefield.
/// The land enters untapped and taps to add {R} to the mana pool.
#[test]
fn riverglide_pathway_back_face_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .hand(0, &[riverglide_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, riverglide_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, riverglide_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, riverglide_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Sandstorm Verge: "{T}: Add {C}." / "{3}, {T}: Target creature can't block this turn. Activate only as a sorcery."
/// Under `Coverage::Partial`, the sorcery-speed restriction preventing blocking is omitted.
/// The Desert taps for colorless mana, adding {C} to the pool and leaving the land tapped.
#[test]
fn sandstorm_verge_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(0, &[sandstorm_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let verge = on_battlefield(&engine, p0, sandstorm_verge()).expect("Verge deployed");
    activate(&mut engine, p0, sandstorm_verge(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, verge));
}

/// Smoldering Spires: "This land enters tapped." / "When this land enters, target creature can't block this turn." / "{T}: Add {R}."
/// Under `Coverage::Partial`, the enters-the-battlefield blocking restriction trigger is omitted.
/// Playing the land from hand verifies that it enters tapped without triggering, and on the next turn it untaps to add {R}.
#[test]
fn smoldering_spires_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(128, forest())
        .hand(0, &[smoldering_spires()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spires = play_land(&mut engine, p0, smoldering_spires());
    assert!(
        entered_tapped(&engine, spires),
        "Smoldering Spires enters tapped"
    );
    assert!(stack_is_empty(&engine), "no enters trigger on the stack");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, spires), "untaps on next turn");

    activate(&mut engine, p0, smoldering_spires(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, spires));
}

/// Soulstone Sanctuary: "{T}: Add {C}." / "{4}: This land becomes a 3/3 creature with vigilance and all creature types. It's still a land."
/// Paid with four Forests, activating ability 1 animates the land indefinitely without tapping it.
/// Upon resolution, the permanent is a 3/3 creature with vigilance and remains an untapped land.
#[test]
fn soulstone_sanctuary_animates_into_creature_with_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(140, forest())
        .battlefield(
            0,
            &[
                soulstone_sanctuary(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctuary = on_battlefield(&engine, p0, soulstone_sanctuary()).expect("Sanctuary deployed");
    assert!(!types(&engine, sanctuary).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, sanctuary);
    activate(&mut engine, p0, soulstone_sanctuary(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, sanctuary);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, sanctuary), (3, 3));
    assert!(keywords(&engine, sanctuary).contains(KeywordSet::VIGILANCE));
    assert!(!is_tapped(&engine, sanctuary));
}

/// Terrain Generator: "{T}: Add {C}." / "{2}, {T}: You may put a basic land card from your hand onto the battlefield tapped."
/// Under `Coverage::Partial`, the hand-to-battlefield basic land drop ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn terrain_generator_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[terrain_generator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let generator = on_battlefield(&engine, p0, terrain_generator()).expect("Generator deployed");
    activate(&mut engine, p0, terrain_generator(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, generator));
}

/// The Tabernacle at Pendrell Vale: "All creatures have 'At the beginning of your upkeep, destroy this creature unless you pay {1}.'"
/// The static ability grants an upkeep trigger to every creature on the battlefield.
/// On turn 1 upkeep, Llanowar Elves' granted trigger resolves; when declining to pay {1}, the creature is destroyed.
#[test]
fn the_tabernacle_at_pendrell_vale_taxes_creatures_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[the_tabernacle_at_pendrell_vale(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until matched PayTax");
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 1 });

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "creature destroyed"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "creature in graveyard"
    );
    // Which object died, said out loud. The sentence is granted to the
    // creature, so `TargetSpec::ThisObject` is the creature and never the
    // land that granted it — and a reading that took the granting permanent
    // instead produced the same visible outcome as the broken one, because
    // both destroyed nothing (#147).
    assert!(
        on_battlefield(&engine, p0, the_tabernacle_at_pendrell_vale()).is_some(),
        "the land that granted the ability destroyed itself"
    );
}

/// Thespian's Stage: "{T}: Add {C}." / "{2}, {T}: This land becomes a copy of target land, except it has this ability."
/// Under `Coverage::Partial`, the target-copying activated ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn thespian_s_stage_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[thespian_s_stage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stage = on_battlefield(&engine, p0, thespian_s_stage()).expect("Stage deployed");
    activate(&mut engine, p0, thespian_s_stage(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, stage));
}

/// Unholy Grotto: "{T}: Add {C}." / "{B}, {T}: Put target Zombie card from your graveyard on top of your library."
/// With a Zombie card in the graveyard and {B} mana available from a Swamp, ability 1 is activated.
/// The targeted Zombie card is returned from the graveyard and placed on top of the library.
#[test]
fn unholy_grotto_puts_zombie_from_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let festering_goblin = card_index("66fb4764-d309-4c30-a2a4-474f9030dc87");
    let mut engine = Duel::new(109, festering_goblin)
        .battlefield(0, &[unholy_grotto(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let grotto = on_battlefield(&engine, p0, unholy_grotto()).expect("Grotto deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let zombie = gy[0];

    tap_mana_except(&mut engine, p0, grotto);
    activate(&mut engine, p0, unholy_grotto(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&zombie));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0,
        "Zombie left the graveyard"
    );
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.last().copied(),
        Some(zombie),
        "Zombie is on top of library"
    );
    assert!(is_tapped(&engine, grotto));
}

/// Unstable Frontier: "{T}: Add {C}." / "{T}: Target land you control becomes the basic land type of your choice until end of turn."
/// Under `Coverage::Partial`, the basic land type grant ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn unstable_frontier_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(0, &[unstable_frontier()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let frontier = on_battlefield(&engine, p0, unstable_frontier()).expect("Frontier deployed");
    activate(&mut engine, p0, unstable_frontier(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, frontier));
}

/// Untaidake, the Cloud Keeper: "Untaidake enters tapped." / "{T}, Pay 2 life: Add {C}{C}. Spend this mana only to cast legendary spells."
/// Activating Untaidake deducts 2 life from the controller and produces two restricted colorless mana.
/// The mana pool receives the restricted mana and the land becomes tapped.
#[test]
fn untaidake_the_cloud_keeper_adds_restricted_mana_paying_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[untaidake_the_cloud_keeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let untaidake =
        on_battlefield(&engine, p0, untaidake_the_cloud_keeper()).expect("Untaidake deployed");
    let life_before = engine.state().players[0].life;

    activate(&mut engine, p0, untaidake_the_cloud_keeper(), 0);

    assert_eq!(engine.state().players[0].life, life_before - 2);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, untaidake));
}

/// Urza's Mine: "{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead."
/// Under `Coverage::Partial`, the multi-permanent Tron replacement condition is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn urza_s_mine_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[urza_s_mine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, urza_s_mine()).expect("Mine deployed");
    activate(&mut engine, p0, urza_s_mine(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, mine));
}

/// Urza's Tower: "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead."
/// Under `Coverage::Partial`, the multi-permanent Tron replacement condition is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn urza_s_tower_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[urza_s_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, urza_s_tower()).expect("Tower deployed");
    activate(&mut engine, p0, urza_s_tower(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tower));
}

/// Vault of the Archangel: "{T}: Add {C}." / "{2}{W}{B}, {T}: Creatures you control gain deathtouch and lifelink until end of turn."
/// Paid with four basic lands, the activated ability resolves to grant all controlled creatures both keywords.
/// Llanowar Elves gains deathtouch and lifelink, and the land remains tapped.
#[test]
fn vault_of_the_archangel_grants_deathtouch_and_lifelink_to_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(
            0,
            &[
                vault_of_the_archangel(),
                plains(),
                swamp(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, vault_of_the_archangel()).expect("Vault deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert!(!keywords(&engine, elves).contains(KeywordSet::DEATHTOUCH));
    assert!(!keywords(&engine, elves).contains(KeywordSet::LIFELINK));

    tap_mana_except(&mut engine, p0, vault);
    activate(&mut engine, p0, vault_of_the_archangel(), 1);

    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, elves);
    assert!(kw.contains(KeywordSet::DEATHTOUCH));
    assert!(kw.contains(KeywordSet::LIFELINK));
    assert!(is_tapped(&engine, vault));
}

/// Wintermoon Mesa: "This land enters tapped." / "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Tap two target lands."
/// Under `Coverage::Partial`, the two-target sacrifice ability is omitted.
/// Playing the land from hand verifies that it enters tapped, and on the next turn it untaps and taps for {C}.
#[test]
fn wintermoon_mesa_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(111, forest())
        .hand(0, &[wintermoon_mesa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mesa = play_land(&mut engine, p0, wintermoon_mesa());
    assert!(
        entered_tapped(&engine, mesa),
        "Wintermoon Mesa enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, mesa), "untaps on next turn");

    activate(&mut engine, p0, wintermoon_mesa(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, mesa));
}

/// Aether Hub: "When this land enters, you get {E}." / "{T}: Add {C}." / "{T}, Pay {E}: Add one mana of any color."
/// Under `Coverage::Partial`, energy counters on players and paying energy are omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and leaves it tapped.
#[test]
fn aether_hub_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[aether_hub()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hub = on_battlefield(&engine, p0, aether_hub()).expect("Aether Hub deployed");
    activate(&mut engine, p0, aether_hub(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hub));
}

/// Agna Qel'a: "This land enters tapped unless you control a basic land." / "{T}: Add {U}." / "{2}{U}, {T}: Draw a card, then discard a card."
/// Controlling a basic Island allows Agna Qel'a to enter untapped from hand.
/// Paying {2}{U} with the three Islands activates its looting ability, drawing a card and asking for a card to discard.
#[test]
fn agna_qel_a_enters_untapped_with_basic_and_loots() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[agna_qel_a()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let agna = play_land(&mut engine, p0, agna_qel_a());
    assert!(
        !entered_tapped(&engine, agna),
        "enters untapped with basic Island"
    );

    tap_mana_except(&mut engine, p0, agna);
    activate(&mut engine, p0, agna_qel_a(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard choice");
    };

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, agna));
}

/// Ancient Amphitheater: "As this land enters, you may reveal a Giant card from your hand. If you don't, this land enters tapped." / "{T}: Add {R} or {W}."
/// Under `Coverage::Partial`, the hand-reveal entry modifier is omitted, allowing the land to enter untapped.
/// Activating Ancient Amphitheater prompts for Red or White and adds {R} to the mana pool.
#[test]
fn ancient_amphitheater_enters_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .hand(0, &[ancient_amphitheater()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ancient_amphitheater());
    assert!(
        !entered_tapped(&engine, land),
        "enters untapped under partial coverage"
    );

    activate(&mut engine, p0, ancient_amphitheater(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Auntie's Hovel: "As this land enters, you may reveal a Goblin card from your hand. If you don't, this land enters tapped." / "{T}: Add {B} or {R}."
/// Under `Coverage::Partial`, the Goblin reveal clause is omitted, so the land enters untapped.
/// Activating Auntie's Hovel prompts for Black or Red and adds {B} to the mana pool.
#[test]
fn auntie_s_hovel_enters_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .hand(0, &[auntie_s_hovel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, auntie_s_hovel());
    assert!(
        !entered_tapped(&engine, land),
        "enters untapped under partial coverage"
    );

    activate(&mut engine, p0, auntie_s_hovel(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Cradle of the Accursed: "{T}: Add {C}." / "{3}, {T}, Sacrifice this land: Create a 2/2 black Zombie creature token."
/// Under `Coverage::Partial`, the Zombie token creation ability is omitted.
/// Activating the implemented ability taps the land for {C} and adds colorless mana to the pool.
#[test]
fn cradle_of_the_accursed_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[cradle_of_the_accursed()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, cradle_of_the_accursed()).expect("Cradle deployed");
    activate(&mut engine, p0, cradle_of_the_accursed(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Dark Fortress: "{T}: Add {C}." / "{T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the conditional {B}/{R} disjunctive ability is omitted.
/// Activating the implemented ability taps the land for {C} and adds colorless mana to the pool.
#[test]
fn dark_fortress_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[dark_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fortress = on_battlefield(&engine, p0, dark_fortress()).expect("Dark Fortress deployed");
    activate(&mut engine, p0, dark_fortress(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, fortress));
}

/// Desert: "{T}: Add {C}." / "{T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step."
/// Under `Coverage::Partial`, the end-of-combat damage ability is omitted due to timing restrictions.
/// Activating Desert produces {C} and leaves the land tapped.
#[test]
fn desert_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest()).battlefield(0, &[desert()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, desert()).expect("Desert deployed");
    activate(&mut engine, p0, desert(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Dunes of the Dead: "{T}: Add {C}." / "When this land is put into a graveyard from the battlefield, create a 2/2 black Zombie creature token."
/// Destroying Dunes of the Dead with Vindicate triggers its death trigger.
/// On resolution, Dunes of the Dead is in the graveyard and a 2/2 Zombie creature token is created on the battlefield.
#[test]
fn dunes_of_the_dead_creates_zombie_token_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[dunes_of_the_dead(), plains(), swamp(), forest()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dunes = on_battlefield(&engine, p0, dunes_of_the_dead()).expect("Dunes deployed");
    tap_mana_except(&mut engine, p0, dunes);

    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Vindicate");
    };
    assert!(options.contains(&dunes));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dunes],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, dunes_of_the_dead()).is_some());
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Zombie token created");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}

/// Gathering Place: "{T}: Add {C}." / "{T}: Add {G} or {W}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land condition.
/// Controlling a basic Forest enables ability 1, prompting for a color choice and adding {W} to the pool.
#[test]
fn gathering_place_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(0, &[gathering_place(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gp = on_battlefield(&engine, p0, gathering_place()).expect("Gathering Place deployed");
    activate(&mut engine, p0, gathering_place(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, gp));
}

/// Gilt-Leaf Palace: "As this land enters, you may reveal an Elf card from your hand. If you don't, this land enters tapped." / "{T}: Add {B} or {G}."
/// Under `Coverage::Partial`, the hand-reveal entry modifier is omitted, so the land enters untapped.
/// Activating the mana ability prompts for Black or Green, and selecting Green adds {G} to the pool.
#[test]
fn gilt_leaf_palace_enters_untapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .hand(0, &[gilt_leaf_palace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = play_land(&mut engine, p0, gilt_leaf_palace());
    assert!(
        !entered_tapped(&engine, palace),
        "enters untapped under partial coverage"
    );

    activate(&mut engine, p0, gilt_leaf_palace(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, palace));
}

/// Gleaming Bastion: "{T}: Add {C}." / "{T}: Add {W} or {U}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land control condition.
/// Controlling a basic Plains enables ability 1, offering White or Blue and adding {U} to the mana pool.
#[test]
fn gleaming_bastion_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(109, plains())
        .battlefield(0, &[gleaming_bastion(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bastion = on_battlefield(&engine, p0, gleaming_bastion()).expect("Bastion deployed");
    activate(&mut engine, p0, gleaming_bastion(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, bastion));
}

/// Glimmervoid: "At the beginning of the end step, if you control no artifacts, sacrifice this land." / "{T}: Add one mana of any color."
/// Under `Coverage::Partial`, the end-step zero-artifact sacrifice trigger is omitted.
/// Activating the land prompts for a color choice and adds one mana of the chosen color ({R}) to the pool.
#[test]
fn glimmervoid_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[glimmervoid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, glimmervoid()).expect("Glimmervoid deployed");
    activate(&mut engine, p0, glimmervoid(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Gods' Eye, Gate to the Reikai: "{T}: Add {C}." / "When Gods' Eye is put into a graveyard from the battlefield, create a 1/1 colorless Spirit creature token."
/// Under `Coverage::Partial`, the Spirit token death trigger is omitted due to a missing token registry entry.
/// Activating the implemented mana ability adds {C} to the pool and leaves Gods' Eye tapped.
#[test]
fn gods_eye_gate_to_the_reikai_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[gods_eye_gate_to_the_reikai()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, gods_eye_gate_to_the_reikai()).expect("Gods Eye deployed");
    activate(&mut engine, p0, gods_eye_gate_to_the_reikai(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Gond Gate: "Gates you control enter untapped." / "{T}: Add {C}." / "{T}: Add one mana of any color that a Gate you control could produce."
/// Under `Coverage::Partial`, the Gate-untap modifier and Gate-dependent colored mana are omitted.
/// Activating Gond Gate's implemented ability adds {C} to the mana pool and taps it.
#[test]
fn gond_gate_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(0, &[gond_gate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, gond_gate()).expect("Gond Gate deployed");
    activate(&mut engine, p0, gond_gate(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Grasping Dunes: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: Put a -1/-1 counter on target creature. Activate only as a sorcery."
/// Paid with a Forest for {1}, the activated ability targets Llanowar Elves and sacrifices Grasping Dunes as a cost.
/// Upon resolution, the -1/-1 counter reduces the 1/1 creature to 0 toughness, sending it to the graveyard.
#[test]
fn grasping_dunes_places_minus_counter_and_destroys_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[grasping_dunes(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dunes = on_battlefield(&engine, p0, grasping_dunes()).expect("Dunes deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");

    tap_mana_except(&mut engine, p0, dunes);
    activate(&mut engine, p0, grasping_dunes(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elves),
        "Llanowar Elves is a valid creature target"
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

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, grasping_dunes()).is_some());
}

/// Great Hall of the Citadel: "{T}: Add {C}." / "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast legendary spells."
/// Paying {1} with a basic Forest and tapping the Great Hall triggers two consecutive color choices.
/// Selecting Red then White adds both restricted mana to the pool and leaves the land tapped.
#[test]
fn great_hall_of_the_citadel_filters_into_two_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(0, &[great_hall_of_the_citadel(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, great_hall_of_the_citadel()).expect("Hall deployed");
    tap_mana_except(&mut engine, p0, hall);

    activate(&mut engine, p0, great_hall_of_the_citadel(), 1);

    let Pending::ChooseColor { options: opt1, .. } = engine.pending().clone() else {
        panic!("expected first color choice, got {:?}", engine.pending());
    };
    assert_eq!(opt1.len(), 5);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let Pending::ChooseColor { options: opt2, .. } = engine.pending().clone() else {
        panic!("expected second color choice, got {:?}", engine.pending());
    };
    assert_eq!(opt2.len(), 5);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    // Read off `restricted()`, not `available()`: the printing says "spend
    // this mana only to cast legendary spells", so it never reaches the
    // plain counters — and asserting it there would pass for a card that
    // had dropped the restriction.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.available(ManaColor::White), 0);
    let mut got: Vec<(ManaColor, u16)> = pool
        .restricted()
        .iter()
        .map(|m| (m.color, m.amount))
        .collect();
    got.sort_by_key(|(color, _)| *color as u8);
    let mut want = vec![(ManaColor::Red, 1u16), (ManaColor::White, 1u16)];
    want.sort_by_key(|(color, _)| *color as u8);
    assert_eq!(got, want);
    assert!(is_tapped(&engine, hall));
}

/// Hall of the Bandit Lord: "Hall of the Bandit Lord enters tapped." / "{T}, Pay 3 life: Add {C}. If that mana is spent on a creature spell, it gains haste."
/// Under `Coverage::Partial`, the haste grant rider on spent mana is omitted.
/// Playing the land verifies it enters tapped; upon untapping next turn, paying 3 life and tapping adds {C} to the pool.
#[test]
fn hall_of_the_bandit_lord_enters_tapped_and_costs_life_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(132, forest())
        .hand(0, &[hall_of_the_bandit_lord()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = play_land(&mut engine, p0, hall_of_the_bandit_lord());
    assert!(entered_tapped(&engine, hall), "enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, hall), "untaps on next turn");

    let life_before = engine.state().players[0].life;
    activate(&mut engine, p0, hall_of_the_bandit_lord(), 0);

    assert_eq!(engine.state().players[0].life, life_before - 3);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hall));
}

/// Haunted Fengraf: "{T}: Add {C}." / "{3}, {T}, Sacrifice this land: Return a creature card at random from your graveyard to your hand."
/// Under `Coverage::Partial`, the random graveyard return ability is omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and taps it.
#[test]
fn haunted_fengraf_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[haunted_fengraf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, haunted_fengraf()).expect("Haunted Fengraf deployed");
    activate(&mut engine, p0, haunted_fengraf(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Hidden Lair: "{T}: Add {C}." / "{T}: Add {U} or {B}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land control condition.
/// Controlling a basic Swamp enables ability 1, offering Blue or Black and adding {U} to the mana pool.
#[test]
fn hidden_lair_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, swamp())
        .battlefield(0, &[hidden_lair(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = on_battlefield(&engine, p0, hidden_lair()).expect("Hidden Lair deployed");
    activate(&mut engine, p0, hidden_lair(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, lair));
}

/// Lotus Field: "Hexproof" / "This land enters tapped." / "When this land enters, sacrifice two lands." / "{T}: Add three mana of any one color."
/// Playing Lotus Field enters tapped with hexproof and triggers an ETB requiring two lands to be sacrificed.
/// On the following turn, Lotus Field untaps and produces three mana of the chosen color ({U}).
#[test]
fn lotus_field_enters_tapped_sacrifices_two_lands_and_adds_three_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[lotus_field()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let f1 = all_on_battlefield(&engine, p0, forest())[0];
    let f2 = all_on_battlefield(&engine, p0, forest())[1];

    let lotus = play_land(&mut engine, p0, lotus_field());
    assert!(entered_tapped(&engine, lotus), "Lotus Field enters tapped");
    assert!(
        keywords(&engine, lotus).contains(KeywordSet::HEXPROOF),
        "Lotus Field has hexproof"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f1] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f2] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 0);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lotus), "untaps on next turn");

    // Index 1: ability 0 is the enters-trigger that sacrificed the two
    // Forests above, and a trigger is never in the activatable set.
    activate(&mut engine, p0, lotus_field(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 3);
    assert!(is_tapped(&engine, lotus));
}

/// Moorland Haunt: "{T}: Add {C}." / "{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying."
/// Under `Coverage::Partial`, the token creation ability with graveyard exile cost is omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and taps Moorland Haunt.
#[test]
fn moorland_haunt_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[moorland_haunt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, moorland_haunt()).expect("Moorland Haunt deployed");
    activate(&mut engine, p0, moorland_haunt(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Mutavault: "{T}: Add {C}." / "{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land."
/// Paying {1} with a Forest activates the animation ability without tapping Mutavault.
/// On resolution, Mutavault is a 2/2 creature that remains a land and is untapped.
#[test]
fn mutavault_animates_into_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[mutavault(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, mutavault()).expect("Mutavault deployed");
    assert!(!types(&engine, vault).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, vault);
    activate(&mut engine, p0, mutavault(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, vault), (2, 2));
    assert!(!is_tapped(&engine, vault));
}

/// Oran-Rief, the Vastwood: "This land enters tapped." / "{T}: Add {G}." / "{T}: Put a +1/+1 counter on each green creature that entered this turn."
/// Under `Coverage::Partial`, the turn-history creature pump is omitted.
/// Playing the land from hand verifies that it enters tapped, and on the following turn it untaps and taps for {G}.
#[test]
fn oran_rief_the_vastwood_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(105, forest())
        .hand(0, &[oran_rief_the_vastwood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, oran_rief_the_vastwood());
    assert!(entered_tapped(&engine, land), "Oran-Rief enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, oran_rief_the_vastwood(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, land));
}

/// Quicksand: "{T}: Add {C}." / "{T}, Sacrifice this land: Target attacking creature without flying gets -1/-2 until end of turn."
/// When an attacking Llanowar Elves without flying is targeted, Quicksand is sacrificed to pay the cost.
/// On resolution, the -1/-2 continuous reduction reduces the 1/1 creature's toughness to -1, destroying it via state-based actions.
#[test]
fn quicksand_sacrifices_to_shrink_and_destroy_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[quicksand(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    let __qs = on_battlefield(&engine, p0, quicksand()).expect("Quicksand deployed");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(p1))],
            },
        )
        .unwrap();

    activate(&mut engine, p0, quicksand(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "attacking elves is legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, quicksand()).is_some());
}

/// Ramunap Ruins: "{T}: Add {C}." / "{T}, Pay 1 life: Add {R}." / "{2}{R}{R}, {T}, Sacrifice a Desert: This land deals 2 damage to each opponent."
/// Paid with four Mountains, activating ability 2 prompts to sacrifice a Desert as cost.
/// Sacrificing Ramunap Ruins itself deals 2 damage to the opponent upon resolution, reducing their life from 20 to 18.
#[test]
fn ramunap_ruins_sacrifices_desert_to_deal_damage_to_opponent() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(127, mountain())
        .battlefield(
            0,
            &[
                ramunap_ruins(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ruins = on_battlefield(&engine, p0, ramunap_ruins()).expect("Ruins deployed");
    tap_mana_except(&mut engine, p0, ruins);

    activate(&mut engine, p0, ramunap_ruins(), 2);

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ruins), "Ramunap Ruins is a Desert");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ruins],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, p0, ramunap_ruins()).is_some());
}

/// Sanctum of Eternity: "{T}: Add {C}." / "{2}, {T}: Return target commander you own from the battlefield to your hand."
/// Under `Coverage::Partial`, the commander-bouncing activation is omitted.
/// Activating Sanctum of Eternity produces {C} and leaves the land tapped.
#[test]
fn sanctum_of_eternity_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[sanctum_of_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, sanctum_of_eternity()).expect("Sanctum deployed");
    activate(&mut engine, p0, sanctum_of_eternity(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Sapseep Forest: "({T}: Add {G}.)" / "This land enters tapped." / "{G}, {T}: You gain 1 life. Activate only if you control two or more green permanents."
/// Controlling two Llanowar Elves satisfies the condition of controlling two or more green permanents.
/// Paying {G} from a Forest and tapping Sapseep Forest gains 1 life upon resolution.
#[test]
fn sapseep_forest_gains_life_when_controlling_two_green_permanents() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(
            0,
            &[
                sapseep_forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sapseep = on_battlefield(&engine, p0, sapseep_forest()).expect("Sapseep deployed");
    let life_before = engine.state().players[0].life;

    tap_mana_except(&mut engine, p0, sapseep);
    activate(&mut engine, p0, sapseep_forest(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, life_before + 1);
    assert!(is_tapped(&engine, sapseep));
}

/// Sea Gate Wreckage: "{T}: Add {C}." / "{2}{C}, {T}: Draw a card. Activate only if you have no cards in hand."
/// Under `Coverage::Partial`, the zero-card hand draw ability is omitted.
/// Activating the land's implemented ability adds {C} to the mana pool and taps Sea Gate Wreckage.
#[test]
fn sea_gate_wreckage_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .battlefield(0, &[sea_gate_wreckage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, sea_gate_wreckage()).expect("Sea Gate Wreckage deployed");
    activate(&mut engine, p0, sea_gate_wreckage(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Secluded Glen: "As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped." / "{T}: Add {U} or {B}."
/// Under `Coverage::Partial`, the hidden-zone Faerie reveal condition is omitted, so the land enters untapped.
/// Activating Secluded Glen prompts for Blue or Black and adds {U} to the mana pool.
#[test]
fn secluded_glen_enters_untapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest()).hand(0, &[secluded_glen()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, secluded_glen());
    assert!(
        !entered_tapped(&engine, land),
        "enters untapped under partial coverage"
    );

    activate(&mut engine, p0, secluded_glen(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Secret Base: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a spell that shares a watermark with this land."
/// Under `Coverage::Partial`, the watermark-restricted mana ability is omitted because watermarks cannot be read by filters.
/// Activating Secret Base produces {C} and leaves the land tapped.
#[test]
fn secret_base_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .battlefield(0, &[secret_base()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, secret_base()).expect("Secret Base deployed");
    activate(&mut engine, p0, secret_base(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Skycoach Waypoint: "{T}: Add {C}." / "{3}, {T}: Target creature becomes prepared."
/// Under `Coverage::Partial`, the target prepare activation is omitted.
/// Activating the implemented mana ability adds {C} to the pool and taps Skycoach Waypoint.
#[test]
fn skycoach_waypoint_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[skycoach_waypoint()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, skycoach_waypoint()).expect("Waypoint deployed");
    activate(&mut engine, p0, skycoach_waypoint(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Stalking Stones: "{T}: Add {C}." / "{6}: This land becomes a 3/3 Elemental artifact creature that's still a land. (This effect lasts indefinitely.)"
/// Paid with six Forests, activating ability 1 animates the land into an artifact creature without tapping it.
/// On resolution, Stalking Stones permanently gains the Creature and Artifact types with 3/3 base P/T.
#[test]
fn stalking_stones_animates_into_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(
            0,
            &[
                stalking_stones(),
                forest(),
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

    let stones = on_battlefield(&engine, p0, stalking_stones()).expect("Stones deployed");
    assert!(!types(&engine, stones).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, stones);
    activate(&mut engine, p0, stalking_stones(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, stones);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, stones), (3, 3));
    assert!(!is_tapped(&engine, stones));
}

/// Thran Quarry: "At the beginning of the end step, if you control no creatures, sacrifice this land." / "{T}: Add one mana of any color."
/// Under `Coverage::Partial`, the negative creature count end-step sacrifice trigger is omitted.
/// Activating the land prompts for a color choice and adds one mana of the chosen color ({G}) to the pool.
#[test]
fn thran_quarry_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[thran_quarry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, thran_quarry()).expect("Thran Quarry deployed");
    activate(&mut engine, p0, thran_quarry(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Training Compound: "{T}: Add {C}." / "{T}: Add {R} or {G}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the conditional {R}/{G} disjunctive ability is omitted.
/// Activating the land produces {C} and leaves Training Compound tapped.
#[test]
fn training_compound_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[training_compound()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, training_compound()).expect("Training Compound deployed");
    activate(&mut engine, p0, training_compound(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// War Room: "{T}: Add {C}." / "{3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card."
/// Under `Coverage::Partial`, the draw ability with commander-identity life cost is omitted.
/// Activating the implemented mana ability adds {C} to the mana pool and taps War Room.
#[test]
fn war_room_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[war_room()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, war_room()).expect("War Room deployed");
    activate(&mut engine, p0, war_room(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Arcane Lighthouse: "{T}: Add {C}." / "{1}, {T}: Until end of turn, creatures your opponents control lose hexproof and shroud and can't have hexproof or shroud."
/// Under `Coverage::Partial`, the continuous effect strips existing hexproof and shroud from opponent creatures until end of turn.
/// Activating ability 1 removes hexproof from an opponent's creature upon resolution.
#[test]
fn arcane_lighthouse_removes_hexproof_from_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[arcane_lighthouse(), forest()])
        .battlefield(1, &[carnage_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tyrant = on_battlefield(&engine, p1, carnage_tyrant()).expect("Carnage Tyrant deployed");
    assert!(keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));

    let lighthouse = on_battlefield(&engine, p0, arcane_lighthouse()).unwrap();
    tap_mana_except(&mut engine, p0, lighthouse);
    activate(&mut engine, p0, arcane_lighthouse(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));
    assert!(is_tapped(&engine, lighthouse));
}

/// Boseiju, Who Shelters All: "Boseiju enters tapped." / "{T}, Pay 2 life: Add {C}. If that mana is spent on an instant or sorcery spell, that spell can't be countered."
/// Under `Coverage::Implemented`, Boseiju enters tapped when played from hand.
/// Paying 2 life and tapping produces one restricted colorless mana.
#[test]
fn boseiju_enters_tapped_and_pays_life_for_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .hand(0, &[boseiju_who_shelters_all()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boseiju = play_land(&mut engine, p0, boseiju_who_shelters_all());
    assert!(entered_tapped(&engine, boseiju));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, boseiju));

    let before_life = engine.state().players[0].life;
    activate(&mut engine, p0, boseiju_who_shelters_all(), 0);

    assert_eq!(engine.state().players[0].life, before_life - 2);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, boseiju));
}

/// Castle Embereth: "Castle Embereth enters tapped unless you control a Mountain." / "{T}: Add {R}." / "{1}{R}{R}, {T}: Creatures you control get +1/+0 until end of turn."
/// Under `Coverage::Implemented`, controlling a Mountain allows Castle Embereth to enter untapped.
/// Activating its second ability for `{1}{R}{R}` and tapping gives creatures you control +1/+0 until end of turn.
#[test]
fn castle_embereth_enters_untapped_with_mountain_and_pumps_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .hand(0, &[castle_embereth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = play_land(&mut engine, p0, castle_embereth());
    assert!(!entered_tapped(&engine, castle));

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("creature deployed");
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_mana_except(&mut engine, p0, castle);
    activate(&mut engine, p0, castle_embereth(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (2, 1));
    assert!(is_tapped(&engine, castle));
}

/// Cathedral of War: "This land enters tapped." / "Exalted" / "{T}: Add {C}."
/// Under `Coverage::Partial`, exalted is unsupported and omitted.
/// The land enters tapped when played, and after untapping taps for colorless mana.
#[test]
fn cathedral_of_war_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .hand(0, &[cathedral_of_war()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cathedral_of_war());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cathedral_of_war(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Choked Estuary: "As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped." / "{T}: Add {U} or {B}."
/// Under `Coverage::Partial`, revealing a card from hand as an entry modifier is unsupported, so the land enters untapped.
/// Activating Choked Estuary offers a choice between Blue and Black mana and adds the chosen mana.
#[test]
fn choked_estuary_enters_untapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .hand(0, &[choked_estuary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, choked_estuary());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, choked_estuary(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Corrupted Crossroads: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a spell with devoid."
/// Under `Coverage::Partial`, the devoid mana restriction is unsupported and that ability is omitted.
/// Activating ability 0 produces one colorless mana and taps the land.
#[test]
fn corrupted_crossroads_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(0, &[corrupted_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crossroads =
        on_battlefield(&engine, p0, corrupted_crossroads()).expect("Crossroads deployed");
    activate(&mut engine, p0, corrupted_crossroads(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, crossroads));
}

/// Eldrazi Temple: "{T}: Add {C}." / "{T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells or activate abilities of colorless Eldrazi."
/// Under `Coverage::Partial`, activating ability 1 adds two restricted colorless mana for colorless Eldrazi spells.
/// The restricted mana appears in `pool.restricted()` rather than general available colorless mana.
#[test]
fn eldrazi_temple_adds_restricted_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[eldrazi_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, eldrazi_temple()).expect("Eldrazi Temple deployed");
    activate(&mut engine, p0, eldrazi_temple(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, temple));
}

/// Forbidden Orchard: "{T}: Add one mana of any color." / "Whenever you tap this land for mana, target opponent creates a 1/1 colorless Spirit creature token."
/// Under `Coverage::Partial`, the tap-for-mana trigger is unsupported and omitted.
/// Activating ability 0 prompts for a color, produces that mana, and taps the land.
#[test]
fn forbidden_orchard_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[forbidden_orchard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let orchard = on_battlefield(&engine, p0, forbidden_orchard()).expect("Orchard deployed");
    activate(&mut engine, p0, forbidden_orchard(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, orchard));
}

/// Foreboding Ruins: "As this land enters, you may reveal a Swamp or Mountain card from your hand. If you don't, this land enters tapped." / "{T}: Add {B} or {R}."
/// Under `Coverage::Partial`, the hand-reveal entry modifier is unsupported, allowing Foreboding Ruins to enter untapped.
/// Activating ability 0 prompts for Black or Red and adds the selected color to the mana pool.
#[test]
fn foreboding_ruins_enters_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(114, forest())
        .hand(0, &[foreboding_ruins()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, foreboding_ruins());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, foreboding_ruins(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Fortified Village: "As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped." / "{T}: Add {G} or {W}."
/// Under `Coverage::Partial`, the hand-reveal entry clause is omitted so Fortified Village enters untapped.
/// Activating the land's mana ability prompts for Green or White and adds `{G}` to the pool.
#[test]
fn fortified_village_enters_untapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .hand(0, &[fortified_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fortified_village());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, fortified_village(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Frostboil Snarl: "As this land enters, you may reveal an Island or Mountain card from your hand. If you don't, this land enters tapped." / "{T}: Add {U} or {R}."
/// Under `Coverage::Partial`, the reveal-from-hand entry modifier is unsupported so Frostboil Snarl enters untapped.
/// Activating ability 0 presents a color choice between Blue and Red, producing `{U}` and tapping the land.
#[test]
fn frostboil_snarl_enters_untapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .hand(0, &[frostboil_snarl()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, frostboil_snarl());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, frostboil_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Furycalm Snarl: "As this land enters, you may reveal a Mountain or Plains card from your hand. If you don't, this land enters tapped." / "{T}: Add {R} or {W}."
/// Under `Coverage::Partial`, the reveal-from-hand clause is unsupported and Furycalm Snarl enters untapped.
/// Activating the land offers Red or White, adding `{R}` and leaving the land tapped.
#[test]
fn furycalm_snarl_enters_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .hand(0, &[furycalm_snarl()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, furycalm_snarl());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, furycalm_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Game Trail: "As this land enters, you may reveal a Mountain or Forest card from your hand. If you don't, this land enters tapped." / "{T}: Add {R} or {G}."
/// Under `Coverage::Partial`, the reveal entry condition is unsupported, allowing Game Trail to enter untapped.
/// Activating ability 0 prompts for Red or Green and adds `{R}` to the mana pool.
#[test]
fn game_trail_enters_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest()).hand(0, &[game_trail()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, game_trail());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, game_trail(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Halimar Depths: "This land enters tapped." / "When this land enters, look at the top three cards of your library, then put them back in any order." / "{T}: Add {U}."
/// Under `Coverage::Implemented`, playing Halimar Depths enters tapped and triggers a reorder of the top three cards.
/// Reversing the offered cards updates their positions in the library.
#[test]
fn halimar_depths_enters_tapped_and_reorders_top_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .hand(0, &[halimar_depths()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, halimar_depths());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::OrderObjects { .. })
    });
    let Pending::OrderObjects { player, objects } = engine.pending().clone() else {
        panic!("expected OrderObjects, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(objects.len(), 3);

    let mut reversed = objects.clone();
    reversed.reverse();
    let expected_top = reversed[0];
    engine
        .apply(p0, PlayerAction::OrderObjects { objects: reversed })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(expected_top)
    );
}

/// Hostile Desert: "{T}: Add {C}." / "{2}, Exile a land card from your graveyard: This land becomes a 3/4 Elemental creature until end of turn. It's still a land."
/// Under `Coverage::Partial`, exiling a land from the graveyard as an activation cost is unsupported, leaving only the mana ability.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn hostile_desert_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[hostile_desert()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let desert = on_battlefield(&engine, p0, hostile_desert()).expect("Hostile Desert deployed");
    activate(&mut engine, p0, hostile_desert(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, desert));
}

/// Leechridden Swamp: "Leechridden Swamp enters tapped." / "{B}, {T}: Each opponent loses 1 life. Activate only if you control two or more black permanents."
/// Under `Coverage::Implemented`, playing this land enters tapped.
/// With two black permanents in play, paying `{B}` and tapping drains 1 life from the opponent.
#[test]
fn leechridden_swamp_enters_tapped_and_drains_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[baleful_strix(), baleful_strix(), swamp()])
        .hand(0, &[leechridden_swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, leechridden_swamp());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let p1_before = engine.state().players[1].life;
    tap_all_mana_but(&mut engine, p0, Some(leechridden_swamp()));
    activate(&mut engine, p0, leechridden_swamp(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, p1_before - 1);
    assert!(is_tapped(&engine, land));
}

/// Madblind Mountain: "Madblind Mountain enters tapped." / "{R}, {T}: Shuffle your library. Activate only if you control two or more red permanents."
/// Under `Coverage::Partial`, no effect shuffles a library alone so the second ability is omitted.
/// Playing this land causes it to enter tapped, and after untapping it taps for red mana.
#[test]
fn madblind_mountain_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .hand(0, &[madblind_mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, madblind_mountain());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, madblind_mountain(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Memorial to Folly: "This land enters tapped." / "{T}: Add {B}." / "{2}{B}, {T}, Sacrifice this land: Return target creature card from your graveyard to your hand."
/// Under `Coverage::Implemented`, Memorial to Folly enters tapped when played from hand.
/// Activating ability 1 sacrifices the land and returns a target creature card from the graveyard to the hand.
#[test]
fn memorial_to_folly_returns_creature_from_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, quiet_creature())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[memorial_to_folly()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, memorial_to_folly());
    assert!(entered_tapped(&engine, land));

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, quiet_creature()).expect("creature in graveyard");

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, memorial_to_folly(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(in_graveyard(&engine, p0, memorial_to_folly()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, quiet_creature()).is_some());
}

/// Mortuary Mire: "This land enters tapped." / "When this land enters, you may put target creature card from your graveyard on top of your library." / "{T}: Add {B}."
/// Under `Coverage::Implemented`, playing Mortuary Mire enters tapped and triggers target selection.
/// Choosing a creature card in the graveyard moves it to the top of the library.
#[test]
fn mortuary_mire_enters_tapped_and_puts_creature_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, quiet_creature())
        .hand(0, &[mortuary_mire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, quiet_creature()).expect("creature in graveyard");

    let land = play_land(&mut engine, p0, mortuary_mire());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(elf)
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_none());
}

/// Mouth of Ronom: "{T}: Add {C}." / "{4}{S}, {T}, Sacrifice this land: It
/// deals 4 damage to target creature." Under `Coverage::Implemented`, paying
/// the whole cost and sacrificing the land deals 4 damage to a target
/// creature, and 4 damage destroys a 1/1 through state-based actions.
///
/// The second Mouth is the `{S}`, and it is there because of a defect rather
/// than because the card wants it: `mana_pay` reads `ManaSymbol::Snow` as
/// `ManaColor::Colorless` (#158), so the symbol is paid here by a colorless
/// source that happens to be snow, and not by a snow source. Four Forests pay
/// the `{4}`. When #158 lands this test keeps passing and stops being a
/// statement about colorless mana; the Forests are the ones to make snow then.
#[test]
fn mouth_of_ronom_sacrifices_to_deal_damage_to_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(132, forest())
        .battlefield(
            0,
            &[
                mouth_of_ronom(),
                mouth_of_ronom(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mouths = all_on_battlefield(&engine, p0, mouth_of_ronom());
    assert_eq!(mouths.len(), 2, "one Mouth to sacrifice and one to tap");
    let (mouth, spare) = (mouths[0], mouths[1]);
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("elf deployed");

    // The four Forests and the spare Mouth, which `tap_mana_except` reaches
    // now that it reads both lists: a nonbasic printing its own `{T}: Add
    // {C}` is an ordinary `(source, index)` entry in `legal.abilities`
    // (#159). The Mouth under test is the one kept back.
    let taken = tap_mana_except(&mut engine, p0, mouth);
    assert_eq!(taken, 5, "four Forests and the spare Mouth");
    assert!(is_tapped(&engine, spare), "the spare Mouth paid for itself");

    activate(&mut engine, p0, mouth_of_ronom(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert!(in_graveyard(&engine, p0, mouth_of_ronom()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none());
}

/// Necroblossom Snarl: "As this land enters, you may reveal a Swamp or Forest card from your hand. If you don't, this land enters tapped." / "{T}: Add {B} or {G}."
/// Under `Coverage::Partial`, the hand reveal clause is omitted and Necroblossom Snarl enters untapped.
/// Activating the mana ability prompts for Black or Green, adding `{B}` and tapping the land.
#[test]
fn necroblossom_snarl_enters_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(119, forest())
        .hand(0, &[necroblossom_snarl()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, necroblossom_snarl());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, necroblossom_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Nesting Grounds: "{T}: Add {C}." / "{1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery."
/// Under `Coverage::Partial`, moving counters between permanents is unsupported, leaving only the mana ability.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn nesting_grounds_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[nesting_grounds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, nesting_grounds()).expect("Nesting Grounds deployed");
    activate(&mut engine, p0, nesting_grounds(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Port Town: "As this land enters, you may reveal a Plains or Island card from your hand. If you don't, this land enters tapped." / "{T}: Add {W} or {U}."
/// Under `Coverage::Partial`, revealing from hand is unsupported and Port Town enters untapped.
/// Activating ability 0 prompts for White or Blue, adding `{W}` and tapping the land.
#[test]
fn port_town_enters_untapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest()).hand(0, &[port_town()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, port_town());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, port_town(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Roadside Reliquary: "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment."
/// Under `Coverage::Partial`, conditional board branching in effects is unsupported and the sacrifice ability is omitted.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn roadside_reliquary_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .battlefield(0, &[roadside_reliquary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, roadside_reliquary()).expect("Roadside Reliquary deployed");
    activate(&mut engine, p0, roadside_reliquary(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Secret Tunnel: "This land can't be blocked." / "{T}: Add {C}." / "{4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn."
/// Under `Coverage::Partial`, the activated ability requiring two targets of shared creature type is omitted.
/// The static ability grants unblockable to the land itself, and activating ability 1 adds `{C}` to the mana pool.
#[test]
fn secret_tunnel_is_unblockable_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[secret_tunnel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = on_battlefield(&engine, p0, secret_tunnel()).expect("Secret Tunnel deployed");
    assert!(keywords(&engine, tunnel).contains(KeywordSet::UNBLOCKABLE));

    activate(&mut engine, p0, secret_tunnel(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tunnel));
}

/// Sequestered Stash: "{T}: Add {C}." / "{4}, {T}, Sacrifice this land: Mill five cards. Then you may put an artifact card from your graveyard on top of your library."
/// Under `Coverage::Partial`, placing an artifact from graveyard on top of the library is unsupported.
/// Activating ability 1 for `{4}` and sacrificing this land mills five cards from the library into the graveyard.
#[test]
fn sequestered_stash_sacrifices_to_mill_five_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(
            0,
            &[sequestered_stash(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stash =
        on_battlefield(&engine, p0, sequestered_stash()).expect("Sequestered Stash deployed");
    let lib_before = library_size(&engine, p0);

    tap_mana_except(&mut engine, p0, stash);
    activate(&mut engine, p0, sequestered_stash(), 1);

    assert!(in_graveyard(&engine, p0, sequestered_stash()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 5);
}

/// Shineshadow Snarl: "As this land enters, you may reveal a Plains or Swamp card from your hand. If you don't, this land enters tapped." / "{T}: Add {W} or {B}."
/// Under `Coverage::Partial`, the hand reveal clause is unsupported and Shineshadow Snarl enters untapped.
/// Activating ability 0 prompts for White or Black, adding `{W}` and tapping the land.
#[test]
fn shineshadow_snarl_enters_untapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .hand(0, &[shineshadow_snarl()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shineshadow_snarl());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, shineshadow_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Shrine of the Forsaken Gods: "{T}: Add {C}." / "{T}: Add {C}{C}. Spend this mana only to cast colorless spells. Activate only if you control seven or more lands."
/// Under `Coverage::Implemented`, controlling seven lands enables the second mana ability.
/// Activating ability 1 adds two restricted colorless mana to `pool.restricted()`.
#[test]
fn shrine_of_the_forsaken_gods_adds_two_restricted_colorless_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(
            0,
            &[
                shrine_of_the_forsaken_gods(),
                forest(),
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

    let shrine =
        on_battlefield(&engine, p0, shrine_of_the_forsaken_gods()).expect("Shrine deployed");
    activate(&mut engine, p0, shrine_of_the_forsaken_gods(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, shrine));
}

/// Springjack Pasture: "{T}: Add {C}." / "{4}, {T}: Create a 0/1 white Goat creature token." / "{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life."
/// Under `Coverage::Partial`, Goat tokens and counted sacrifice costs are unsupported, leaving only ability 0.
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn springjack_pasture_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(0, &[springjack_pasture()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pasture =
        on_battlefield(&engine, p0, springjack_pasture()).expect("Springjack Pasture deployed");
    activate(&mut engine, p0, springjack_pasture(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, pasture));
}

/// Starting Town: "This land enters tapped unless it's your first, second, or third turn of the game." / "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color."
/// Under `Coverage::Partial`, counting turns of the game is unsupported and the land enters untapped.
/// Activating ability 1 costs 1 life and produces one mana of any color chosen by the player.
#[test]
fn starting_town_enters_untapped_and_pays_life_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest()).hand(0, &[starting_town()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, starting_town());
    assert!(!entered_tapped(&engine, land));

    let before_life = engine.state().players[0].life;
    activate(&mut engine, p0, starting_town(), 1);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert_eq!(engine.state().players[0].life, before_life - 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Tectonic Edge: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: Destroy target nonbasic land. Activate only if an opponent controls four or more lands."
/// Under `Coverage::Partial`, counting opponent lands is not supported so the destroy ability is ungated.
/// Activating the sacrifice ability targets and destroys an opponent's nonbasic land.
#[test]
fn tectonic_edge_destroys_target_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(109, forest())
        .battlefield(0, &[tectonic_edge(), forest()])
        .battlefield(1, &[irrigated_farmland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land =
        on_battlefield(&engine, p1, irrigated_farmland()).expect("target land deployed");
    let edge = on_battlefield(&engine, p0, tectonic_edge()).expect("the Edge stands");
    tap_mana_except(&mut engine, p0, edge);

    activate(&mut engine, p0, tectonic_edge(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, tectonic_edge()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, irrigated_farmland()).is_some());
    assert!(on_battlefield(&engine, p1, irrigated_farmland()).is_none());
}

/// The Gold Saucer: "{T}: Add {C}." / "{2}, {T}: Flip a coin. If you win the flip, create a Treasure token." / "{3}, {T}, Sacrifice two artifacts: Draw a card."
/// Under `Coverage::Partial`, flipping a coin and sacrificing multiple artifacts are unsupported, leaving only the mana ability.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn the_gold_saucer_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(0, &[the_gold_saucer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let saucer = on_battlefield(&engine, p0, the_gold_saucer()).expect("The Gold Saucer deployed");
    activate(&mut engine, p0, the_gold_saucer(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, saucer));
}

/// The Grey Havens: "When The Grey Havens enters, scry 1." / "{T}: Add {C}." / "{T}: Add one mana of any color among legendary creature cards in your graveyard."
/// Under `Coverage::Partial`, reading colors from cards in a graveyard is unsupported.
/// Playing The Grey Havens triggers scry 1, and activating ability 1 adds one colorless mana to the pool.
#[test]
fn the_grey_havens_triggers_scry_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .hand(0, &[the_grey_havens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_grey_havens());
    assert!(!entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::ScryBottom,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ScryBottom choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, the_grey_havens(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Undiscovered Paradise: "{T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand."
/// Under `Coverage::Partial`, the delayed return to hand at untap is unsupported.
/// Activating ability 0 prompts for a mana color and adds the chosen mana to the pool.
#[test]
fn undiscovered_paradise_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[undiscovered_paradise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let paradise = on_battlefield(&engine, p0, undiscovered_paradise()).expect("Paradise deployed");
    activate(&mut engine, p0, undiscovered_paradise(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, paradise));
}

/// Urza's Workshop: "{T}: Add {C}." / "Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts."
/// Under `Coverage::Implemented`, controlling three artifacts enables the dynamic metalcraft mana ability.
/// With three Urza's lands in play, activating ability 1 adds three colorless mana to the pool.
#[test]
fn urza_s_workshop_adds_mana_for_each_urza_land_with_metalcraft() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(
            0,
            &[
                urza_s_workshop(),
                urza_s_mine(),
                urza_s_tower(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let workshop = on_battlefield(&engine, p0, urza_s_workshop()).expect("Workshop deployed");
    activate(&mut engine, p0, urza_s_workshop(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 3);
    assert!(is_tapped(&engine, workshop));
}

/// Vineglimmer Snarl: "As this land enters, you may reveal a Forest or Island card from your hand. If you don't, this land enters tapped." / "{T}: Add {G} or {U}."
/// Under `Coverage::Partial`, the hand-reveal entry modifier is unsupported and Vineglimmer Snarl enters untapped.
/// Activating ability 0 prompts for Green or Blue, adding `{G}` and tapping the land.
#[test]
fn vineglimmer_snarl_enters_untapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .hand(0, &[vineglimmer_snarl()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, vineglimmer_snarl());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, vineglimmer_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Wanderwine Hub: "As this land enters, you may reveal a Merfolk card from your hand. If you don't, this land enters tapped." / "{T}: Add {W} or {U}."
/// Under `Coverage::Partial`, the reveal-from-hand entry clause is unsupported and Wanderwine Hub enters untapped.
/// Activating ability 0 prompts for White or Blue, adding `{W}` and tapping the land.
#[test]
fn wanderwine_hub_enters_untapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, forest())
        .hand(0, &[wanderwine_hub()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wanderwine_hub());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, wanderwine_hub(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Zoetic Cavern: "{T}: Add {C}." / "Morph {2}"
/// Under `Coverage::Partial`, casting face-down and turning face up with morph are unsupported.
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn zoetic_cavern_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[zoetic_cavern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cavern = on_battlefield(&engine, p0, zoetic_cavern()).expect("Zoetic Cavern deployed");
    activate(&mut engine, p0, zoetic_cavern(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cavern));
}

/// Adagia, Windswept Bastion: "This land enters tapped." / "{T}: Add {W}." / "Station..." / "12+ | {3}{W}, {T}: Create a token that's a copy of target artifact or enchantment you control, except it's legendary."
/// Under `Coverage::Partial`, Station and the `12+` copy ability are omitted because power-scaled counters and legendary copy modifications are not supported.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn adagia_windswept_bastion_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(218, forest())
        .hand(0, &[adagia_windswept_bastion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, adagia_windswept_bastion());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, adagia_windswept_bastion(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Avengers Tower: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Hero spell..." / "{4}, {T}: Look at the top three cards..."
/// Under `Coverage::Partial`, both Hero-restricted abilities are omitted because hero source restrictions and filtered picks are inexpressible.
/// Playing Avengers Tower enters untapped and immediately taps for colorless mana.
#[test]
fn avengers_tower_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(223, forest())
        .hand(0, &[avengers_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, avengers_tower());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, avengers_tower(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Ba Sing Se: "This land enters tapped unless you control a basic land." / "{T}: Add {G}." / "{2}{G}, {T}: Earthbend 2. Activate only as a sorcery."
/// Under `Coverage::Partial`, controlling a basic land allows Ba Sing Se to enter untapped.
/// Activating ability 1 animates a target land into a creature with haste and places two `+1/+1` counters on it.
#[test]
fn ba_sing_se_enters_untapped_and_animates_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(206, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[ba_sing_se()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ba_sing_se());
    assert!(!entered_tapped(&engine, land));

    // The Forests pay; Ba Sing Se stays up, because `tap_all_mana` takes its
    // printed `{T}: Add` too (#159) and the animation is its other ability.
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, ba_sing_se(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 2));
    assert!(keywords(&engine, land).contains(KeywordSet::HASTE));
    assert!(is_tapped(&engine, land));
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

/// Balamb Garden, `SeeD` Academy // Balamb Garden, Airborne: "This land enters tapped." / "{T}: Add {G} or {U}." / "{5}{G}{U}, {T}: Transform this land..."
/// Under `Coverage::Partial`, the transform ability and back-face crew ability are omitted.
/// Playing the front face enters tapped, and after untapping on a subsequent turn it offers a choice between green and blue mana.
#[test]
fn balamb_garden_enters_tapped_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(222, forest())
        .hand(0, &[balamb_garden_see_d_academy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balamb_garden_see_d_academy());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, balamb_garden_see_d_academy(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Bucolic Ranch: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Mount spell." / "{3}, {T}: Look at the top card of your library..."
/// Under `Coverage::Partial`, the top-card inspection ability is omitted because no effect offers a conditional keep-or-bottom.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn bucolic_ranch_adds_restricted_mount_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(209, forest())
        .battlefield(0, &[bucolic_ranch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ranch = on_battlefield(&engine, p0, bucolic_ranch()).expect("ranch deployed");
    activate(&mut engine, p0, bucolic_ranch(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert!(is_tapped(&engine, ranch));
}

/// Chocobo Camp: "This land enters tapped unless you control a legendary creature." / "{T}: Add {G}." / "{2}{G}{G}, {T}: Create a 2/2 green Bird creature token..."
/// Under `Coverage::Partial`, the delayed counter rider and token creation are omitted.
/// Controlling a legendary creature allows Chocobo Camp to enter untapped and immediately tap for green mana.
#[test]
fn chocobo_camp_enters_untapped_with_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(0, &[katara_the_fearless()])
        .hand(0, &[chocobo_camp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, chocobo_camp());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, chocobo_camp(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Demolition Field: "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls..."
/// Under `Coverage::Partial`, the opponent's basic land enters tapped via `Effect::OptionalBasicLandSearchFor`.
/// Activating ability 1 destroys an opponent's target nonbasic land and puts Demolition Field into its owner's graveyard.
#[test]
fn demolition_field_destroys_opponent_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(210, forest())
        .battlefield(0, &[demolition_field(), forest(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, badlands()).expect("Badlands deployed");
    let field = on_battlefield(&engine, p0, demolition_field()).expect("Field deployed");

    tap_mana_except(&mut engine, p0, field);
    activate(&mut engine, p0, demolition_field(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, demolition_field()).is_some());

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for search: {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 basic search, got {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 basic search, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert!(on_battlefield(&engine, p1, badlands()).is_none());
}

/// Diamond City: "This land enters with a shield counter on it." / "{T}: Add {C}." / "{T}: Move a shield counter from this land onto target creature..."
/// Under `Coverage::Partial`, shield counter clauses are omitted because `CounterKind` has no shield counter variant.
/// Playing Diamond City enters untapped with no counters and taps immediately for colorless mana.
#[test]
fn diamond_city_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(221, forest()).hand(0, &[diamond_city()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, diamond_city());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, diamond_city(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Forsaken Crossroads: "Forsaken Crossroads enters the battlefield tapped." / "As Forsaken Crossroads enters the battlefield, choose a color." / "When Forsaken Crossroads enters the battlefield, scry 1..." / "{T}: Add one mana of the chosen color."
/// Under `Coverage::Partial`, the non-starting player untap replacement is omitted.
/// Entering prompts for a color and scries 1 while entering tapped, and after untapping it taps for the chosen color.
#[test]
fn forsaken_crossroads_enters_tapped_chooses_color_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(224, forest())
        .hand(0, &[forsaken_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, forsaken_crossroads()).expect("crossroads in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice on entry, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    // The enters trigger scries 1, and it is mandatory here: the "you may
    // untap instead" half is the clause `Coverage::Partial` names.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::ScryBottom,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(entered_tapped(&engine, card));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, card));

    activate(&mut engine, p0, forsaken_crossroads(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, card));
}

/// Frostwalk Bastion: "{T}: Add {C}." / "{1}{S}: Until end of turn, this land becomes a 2/3 Construct artifact creature. It's still a land."
/// Under `Coverage::Partial`, the combat-damage trigger is omitted because `Trigger` has no variant for damage to a creature.
/// Paying `{1}{S}` animates the land into a 2/3 Construct artifact creature while retaining its land type.
#[test]
fn frostwalk_bastion_animates_into_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(214, forest())
        .battlefield(0, &[frostwalk_bastion(), mouth_of_ronom(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bastion = on_battlefield(&engine, p0, frostwalk_bastion()).expect("Bastion deployed");
    // `{1}{S}` wants a colorless mana and the Forest cannot make one; the
    // Bastion could, but its animation cost has no `{T}`, so paying with
    // itself would leave it tapped and the last assertion below is that it
    // is not. The Mouth is the snow source and `tap_mana_except` now taps it
    // with the rest (#159) — it stays the right board once `{S}` means
    // "from a snow source" (#158).
    let mouth = on_battlefield(&engine, p0, mouth_of_ronom()).expect("the Mouth stands");
    tap_mana_except(&mut engine, p0, bastion);
    assert!(
        is_tapped(&engine, mouth),
        "the Mouth made the colorless mana"
    );
    activate(&mut engine, p0, frostwalk_bastion(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bastion), (2, 3));
    let types = engine
        .state()
        .object(bastion)
        .unwrap()
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::ARTIFACT));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, bastion));
}

/// Glacial Chasm: "Cumulative upkeep—Pay 2 life." / "When this land enters, sacrifice a land." / "Creatures you control can't attack." / "Prevent all damage that would be dealt to you."
/// Under `Coverage::Partial`, cumulative upkeep, attack prevention, and damage prevention are omitted.
/// When Glacial Chasm enters the battlefield, its enters-trigger forces its controller to sacrifice a land.
#[test]
fn glacial_chasm_sacrifices_a_land_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(225, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[glacial_chasm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_obj = on_battlefield(&engine, p0, forest()).expect("Forest deployed");
    let chasm = play_land(&mut engine, p0, glacial_chasm());
    assert!(!entered_tapped(&engine, chasm));

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for trigger: {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected land sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(options.contains(&forest_obj));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_obj],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, glacial_chasm()).is_some());
}

/// Great Hall of the Biblioplex: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."
/// Under `Coverage::Partial`, the `{5}` animation ability is omitted because no effect animates the source without a target.
/// Activating ability 1 pays 1 life and adds one restricted mana of the chosen color to `pool.restricted()`.
#[test]
fn great_hall_of_the_biblioplex_adds_restricted_spell_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(215, forest())
        .battlefield(0, &[great_hall_of_the_biblioplex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, great_hall_of_the_biblioplex()).expect("Hall deployed");
    activate(&mut engine, p0, great_hall_of_the_biblioplex(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert!(is_tapped(&engine, hall));
}

/// The Forgotten Realms creature-lands and Thran Portal, played from hand
/// on both sides of their own bound.
///
/// All four joined `LANDS_THAT_WOULD_COUNT_THEMSELVES` when they were
/// written, because their filter matches the land that is arriving — so the
/// only thing keeping them from turning off one land early is
/// `controls_at_least` skipping the entering permanent, and the list says a
/// new arrival owes a played test.
///
/// Both sides of every bound, because a test of one side passes against a
/// clause that is simply always true or always false. And the bound is not
/// the same for all four: the manlands print "if you control two or more
/// **other** lands, this land enters tapped" (`at_most: 1`) and Thran
/// Portal prints "unless you control two or fewer other lands"
/// (`at_most: 2`), which is the same predicate from opposite ends.
///
/// Three of these four also carried the defect that
/// `every_counted_entry_clause_is_scoped_to_its_controller` now lints: a
/// bare `Filter::LAND` counts the lands **across the table**, because
/// `controls_count` scopes nothing. So the untapped half of each pair is
/// set up with lands on the opponent's side of the table as well, which is
/// the board that told the two apart.
#[test]
fn a_counted_entry_clause_counts_only_its_controller_s_other_lands() {
    let p0 = PlayerId::new(0);
    // (the land, how many of my lands still let it enter untapped)
    let cases: [(CardIndex, usize); 4] = [
        (hall_of_storm_giants(), 1),
        (den_of_the_bugbear(), 1),
        (hive_of_the_eye_tyrant(), 1),
        (thran_portal(), 2),
    ];

    for (land, at_most) in cases {
        // At the bound, and with a full opposing board: untapped, and
        // counting the other seat's lands would tap it.
        let mine = vec![island(); at_most];
        let mut engine = Duel::new(217, forest())
            .battlefield(0, &mine)
            .battlefield(1, &[island(), island(), island(), island()])
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(
            !entered_tapped(&engine, card),
            "at the bound with four lands across the table, it enters untapped"
        );

        // One over the bound, on my own side: tapped.
        let mine = vec![island(); at_most + 1];
        let mut engine = Duel::new(218, forest())
            .battlefield(0, &mine)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, card),
            "one land over the bound, it enters tapped"
        );
    }
}

/// Hall of Storm Giants: "If you control two or more other lands, this land enters tapped." / "{T}: Add {U}." / "{5}{U}: Until end of turn, this land becomes a 7/7 blue Giant creature with ward {3}. It's still a land."
/// Under `Coverage::Partial`, ward `{3}` is omitted from the animation effect because no modifier grants ward.
/// After untapping, paying `{5}{U}` animates the land into a 7/7 blue Giant creature that remains a land.
#[test]
fn hall_of_storm_giants_animates_into_giant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(216, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[hall_of_storm_giants()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = play_land(&mut engine, p0, hall_of_storm_giants());
    assert!(entered_tapped(&engine, hall));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, hall));

    tap_mana_except(&mut engine, p0, hall);
    activate(&mut engine, p0, hall_of_storm_giants(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, hall), (7, 7));
    let types = engine.state().object(hall).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, hall));
}

/// Havengul Laboratory // Havengul Mystery: "{T}: Add {C}." / "{4}, {T}: Investigate." / "At the beginning of your end step, if you sacrificed three or more Clues this turn, transform..."
/// Under `Coverage::Partial`, the transform loop and back-face reanimation trigger are omitted.
/// Paying `{4}` and tapping Havengul Laboratory activates Investigate, creating a Clue token on resolution.
#[test]
fn havengul_laboratory_investigates() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(226, forest())
        .battlefield(
            0,
            &[
                havengul_laboratory(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, havengul_laboratory()).expect("Lab deployed");
    tap_mana_except(&mut engine, p0, lab);
    activate(&mut engine, p0, havengul_laboratory(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, p0).len(), 1);
    assert!(is_tapped(&engine, lab));
}

/// Hidden Cataract: "This land enters tapped." / "{T}: Add {U}." / "{4}{U}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because no effect performs discover.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn hidden_cataract_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(201, forest())
        .hand(0, &[hidden_cataract()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_cataract());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_cataract(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Hidden Courtyard: "This land enters tapped." / "{T}: Add {W}." / "{4}{W}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because discover is not expressible in the `DSL`.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn hidden_courtyard_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(202, forest())
        .hand(0, &[hidden_courtyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_courtyard());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_courtyard(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Hidden Necropolis: "This land enters tapped." / "{T}: Add {B}." / "{4}{B}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because no effect performs discover.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn hidden_necropolis_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(203, forest())
        .hand(0, &[hidden_necropolis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_necropolis());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_necropolis(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Hidden Nursery: "This land enters tapped." / "{T}: Add {G}." / "{4}{G}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because no effect performs discover.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn hidden_nursery_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(204, forest())
        .hand(0, &[hidden_nursery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_nursery());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_nursery(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Hidden Volcano: "This land enters tapped." / "{T}: Add {R}." / "{4}{R}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Under `Coverage::Partial`, the discover ability is omitted because no effect performs discover.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for red mana.
#[test]
fn hidden_volcano_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(205, forest())
        .hand(0, &[hidden_volcano()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_volcano());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_volcano(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Hostile Hostel // Creeping Inn: "{T}: Add {C}." / "{1}, {T}, Sacrifice a creature: Put a soul counter on this land..."
/// Under `Coverage::Partial`, the soul-counter transform and linked-exile attack trigger are omitted.
/// Playing the front face enters untapped and immediately taps for colorless mana.
#[test]
fn hostile_hostel_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(227, forest())
        .hand(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hostile_hostel());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hostile_hostel(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Kavaron, Memorial World: "This land enters tapped." / "{T}: Add {R}." / "Station..." / "12+ | {1}{R}, {T}, Sacrifice a land: Create a 2/2 colorless Robot artifact creature token..."
/// Under `Coverage::Partial`, Station is omitted because no amount reads the power of a creature tapped to pay a cost.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for red mana.
#[test]
fn kavaron_memorial_world_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(219, forest())
        .hand(0, &[kavaron_memorial_world()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, kavaron_memorial_world());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, kavaron_memorial_world(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Lupinflower Village: "{T}: Add {C}." / "{T}: Add {W}. Spend this mana only to cast a creature spell." / "{1}{W}, {T}, Sacrifice this land: Look at the top six cards..."
/// Under `Coverage::Partial`, the `{1}{W}` search ability is omitted because `Effect::LookAtTopPick` carries no type filter.
/// Activating ability 1 adds one restricted white mana spendable only on creature spells to `pool.restricted()`.
#[test]
fn lupinflower_village_adds_restricted_creature_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(228, forest())
        .battlefield(0, &[lupinflower_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, lupinflower_village()).expect("Village deployed");
    activate(&mut engine, p0, lupinflower_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert!(is_tapped(&engine, village));
}

/// Monumental Henge: "This land enters tapped unless you control a Plains." / "{T}: Add {W}." / "{2}{W}{W}, {T}: Look at the top five cards of your library..."
/// Under `Coverage::Partial`, the `{2}{W}{W}` look-and-pick ability is omitted because `Effect::LookAtTopPick` carries no filter.
/// Controlling a Plains allows Monumental Henge to enter untapped and immediately tap for white mana.
#[test]
fn monumental_henge_enters_untapped_with_plains_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(207, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[monumental_henge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, monumental_henge());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, monumental_henge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Mosswort Bridge: "Hideaway 4" / "This land enters tapped." / "{T}: Add {G}." / "{G}, {T}: You may play the exiled card without paying its mana cost if creatures you control have total power 10 or greater."
/// Under `Coverage::Partial`, Hideaway 4 and the conditional free cast are omitted because face-down exile and total-power conditions are inexpressible.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn mosswort_bridge_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(229, forest())
        .hand(0, &[mosswort_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mosswort_bridge());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, mosswort_bridge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Shelldock Isle: "Hideaway 4" / "This land enters tapped." / "{T}: Add {U}." / "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it."
/// Under `Coverage::Partial`, Hideaway 4 and the library-size conditional cast are omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn shelldock_isle_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(230, forest())
        .hand(0, &[shelldock_isle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shelldock_isle());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, shelldock_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Spawning Pool: "This land enters tapped." / "{T}: Add {B}." / "{1}{B}: This land becomes a 1/1 black Skeleton creature with '{B}: Regenerate this creature' until end of turn. It's still a land."
/// Under `Coverage::Partial`, the regenerate grant is omitted because no regenerate effect or keyword exists in the engine.
/// Playing this land enters tapped, and paying `{1}{B}` after untapping turns it into a 1/1 black Skeleton creature that remains a land.
#[test]
fn spawning_pool_enters_tapped_and_animates_into_skeleton() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(217, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[spawning_pool()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pool = play_land(&mut engine, p0, spawning_pool());
    assert!(entered_tapped(&engine, pool));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, pool));

    tap_mana_except(&mut engine, p0, pool);
    activate(&mut engine, p0, spawning_pool(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, pool), (1, 1));
    let types = engine.state().object(pool).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, pool));
}

/// Spymaster's Vault: "This land enters tapped unless you control a Swamp." / "{T}: Add {B}." / "{B}, {T}: Target creature you control connives X..."
/// Under `Coverage::Partial`, the connive ability is omitted because connive has no effect in the `DSL`.
/// Controlling a Swamp allows Spymaster's Vault to enter untapped and immediately tap for black mana.
#[test]
fn spymaster_s_vault_enters_untapped_with_swamp_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(208, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[spymaster_s_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, spymaster_s_vault());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, spymaster_s_vault(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Susur Secundi, Void Altar: "This land enters tapped." / "{T}: Add {B}." / "Station..." / "12+ | {1}{B}, {T}, Pay 2 life, Sacrifice a creature: Draw cards equal to the sacrificed creature's power."
/// Under `Coverage::Partial`, Station and the `12+` draw ability are omitted because no amount reads the power of a creature tapped or sacrificed in a cost.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn susur_secundi_void_altar_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(220, forest())
        .hand(0, &[susur_secundi_void_altar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, susur_secundi_void_altar());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, susur_secundi_void_altar(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// The World Tree: "This land enters tapped." / "{T}: Add {G}." / "As long as you control six or more lands, lands you control have '{T}: Add one mana of any color.'"
/// Under `Coverage::Partial`, the conditional land grant and the unbounded God search are omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn the_world_tree_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(211, forest())
        .hand(0, &[the_world_tree()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_world_tree());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, the_world_tree(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Villainous Hideout: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Villain spell..." / "{3}, {T}: Target Villain you control connives."
/// Under `Coverage::Partial`, the connive ability is omitted because connive has no effect representation.
/// Activating ability 1 adds one restricted mana of any chosen color to `pool.restricted()`.
#[test]
fn villainous_hideout_adds_restricted_villain_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(231, forest())
        .battlefield(0, &[villainous_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hideout = on_battlefield(&engine, p0, villainous_hideout()).expect("Hideout deployed");
    activate(&mut engine, p0, villainous_hideout(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert!(is_tapped(&engine, hideout));
}

/// Voldaren Estate: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a Vampire spell." / "{5}, {T}: Create a Blood token..."
/// Under `Coverage::Partial`, the Blood token ability is omitted because the Vampire cost reduction is inexpressible.
/// Activating ability 1 pays 1 life and adds one restricted mana to `pool.restricted()`.
#[test]
fn voldaren_estate_pays_life_for_restricted_vampire_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(212, forest())
        .battlefield(0, &[voldaren_estate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let estate = on_battlefield(&engine, p0, voldaren_estate()).expect("Estate deployed");
    activate(&mut engine, p0, voldaren_estate(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Black);
    assert!(is_tapped(&engine, estate));
}

/// Windbrisk Heights: "Hideaway 4" / "This land enters tapped." / "{T}: Add {W}." / "{W}, {T}: You may play the exiled card without paying its mana cost if you attacked with three or more creatures this turn."
/// Under `Coverage::Partial`, Hideaway 4 and the attack-count conditional cast are omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn windbrisk_heights_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(232, forest())
        .hand(0, &[windbrisk_heights()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, windbrisk_heights());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, windbrisk_heights(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Archway of Innovation: "This land enters tapped unless you control an Island." / "{T}: Add {U}." / "{U}, {T}: The next spell you cast this turn has improvise."
/// Under `Coverage::Partial`, the improvise grant is omitted because no modifier grants improvise.
/// Controlling an Island allows Archway of Innovation to enter untapped and immediately tap for blue mana.
#[test]
fn archway_of_innovation_enters_untapped_with_island_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, forest())
        .battlefield(0, &[island()])
        .hand(0, &[archway_of_innovation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, archway_of_innovation());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, archway_of_innovation(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Arena of Glory: "This land enters tapped unless you control a Mountain." / "{T}: Add {R}." / "{R}, {T}, Exert this land: Add {R}{R}..."
/// Under `Coverage::Partial`, the exert ability is omitted because no cost part exerts a permanent.
/// Controlling a Mountain allows Arena of Glory to enter untapped and immediately tap for red mana.
#[test]
fn arena_of_glory_enters_untapped_with_mountain_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[arena_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, arena_of_glory());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, arena_of_glory(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Balduvian Trading Post: "If this land would enter, sacrifice an untapped Mountain instead..." / "{T}: Add {C}{R}." / "{1}, {T}: This land deals 1 damage to target attacking creature."
/// Under `Coverage::Partial`, the entry replacement sacrificing an untapped Mountain is omitted.
/// Playing this land allows it to enter untapped and immediately tap for one colorless and one red mana.
#[test]
fn balduvian_trading_post_taps_for_colorless_and_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(330, forest())
        .hand(0, &[balduvian_trading_post()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balduvian_trading_post());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, balduvian_trading_post(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Blast Zone: "This land enters with a charge counter on it." / "{T}: Add {C}." / "{X}{X}, {T}: Put X charge counters on this land..."
/// Under `Coverage::Partial`, the `{3}, {T}` permanent destruction ability is omitted.
/// Playing this land causes it to enter with a charge counter and immediately tap for colorless mana.
#[test]
fn blast_zone_enters_with_charge_counter_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(331, forest()).hand(0, &[blast_zone()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, blast_zone());
    assert_eq!(counters_on(&engine, land, CounterKind::Charge), 1);

    activate(&mut engine, p0, blast_zone(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Branch of Vitu-Ghazi: "{T}: Add {C}." / "Disguise {3}..." / "When this land is turned face up, add two mana of any one color..."
/// Under `Coverage::Partial`, disguise and the turned-face-up ability are omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn branch_of_vitu_ghazi_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(332, forest())
        .hand(0, &[branch_of_vitu_ghazi()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, branch_of_vitu_ghazi());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, branch_of_vitu_ghazi(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Cactus Preserve: "This land enters tapped." / "{T}: Add one mana of any type that a land you control could produce." / "{3}: Until end of turn, this land becomes an X/X green Plant creature..."
/// Under `Coverage::Partial`, the `{3}` animation is omitted because no amount reads commander mana values.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it produces red mana matching a controlled Mountain.
#[test]
fn cactus_preserve_enters_tapped_and_taps_for_controlled_land_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[cactus_preserve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cactus_preserve());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cactus_preserve(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Clive's Hideaway: "Hideaway 4..." / "{T}: Add {C}." / "{2}, {T}: You may play the exiled card..."
/// Under `Coverage::Partial`, Hideaway 4 and playing the exiled card are omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn clive_s_hideaway_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(329, forest())
        .hand(0, &[clive_s_hideaway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, clive_s_hideaway());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, clive_s_hideaway(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Country Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {W}." / "{1}{W}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn country_roads_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(322, forest()).hand(0, &[country_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, country_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, country_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Dalkovan Encampment: "This land enters tapped unless you control a Swamp or a Mountain." / "{T}: Add {W}." / "{2}{W}, {T}: Whenever you attack this turn..."
/// Under `Coverage::Partial`, the `{2}{W}` delayed attack trigger is omitted because no effect creates attacking delayed triggers.
/// Controlling a Swamp allows Dalkovan Encampment to enter untapped and immediately tap for white mana.
#[test]
fn dalkovan_encampment_enters_untapped_with_swamp_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(304, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[dalkovan_encampment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, dalkovan_encampment());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, dalkovan_encampment(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// Den of the Bugbear: "If you control two or more other lands, this land enters tapped." / "{T}: Add {R}." / "{3}{R}: Until end of turn, this land becomes a 3/2 red Goblin creature..."
/// Under `Coverage::Partial`, the attack trigger creating an attacking Goblin token is omitted.
/// Playing this land while controlling two or more other lands enters tapped, and paying `{3}{R}` after untapping animates it into a 3/2 Goblin creature.
#[test]
fn den_of_the_bugbear_animates_into_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(310, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[den_of_the_bugbear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, den_of_the_bugbear());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, den_of_the_bugbear(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 2));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Eclipsed Realms: "As this land enters, choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk." / "{T}: Add {C}." / "{T}: Add one mana of any color..."
/// Under `Coverage::Partial`, the printed spend restriction and restricted list of eight creature types are omitted.
/// Playing this land asks for a subtype choice as it enters, and activating ability 1 adds one mana of any chosen color.
#[test]
fn eclipsed_realms_chooses_subtype_on_entry_and_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(308, forest())
        .hand(0, &[eclipsed_realms()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, eclipsed_realms()).expect("land in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected subtype choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine
        .apply(p0, PlayerAction::ChooseSubtype(options[0]))
        .unwrap();

    let land = on_battlefield(&engine, p0, eclipsed_realms()).expect("land on battlefield");
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, eclipsed_realms(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Fertile Thicket: "This land enters tapped." / "When this land enters, you may look at the top five cards of your library..." / "{T}: Add {G}."
/// Under `Coverage::Partial`, the enters trigger searching the top five cards is omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn fertile_thicket_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(333, forest())
        .hand(0, &[fertile_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fertile_thicket());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, fertile_thicket(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Foul Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {B}." / "{1}{B}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn foul_roads_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(323, forest()).hand(0, &[foul_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, foul_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, foul_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Gallifrey Council Chamber: "When Gallifrey Council Chamber enters, surveil 1." / "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Time Lord or Alien spell..."
/// Under `Coverage::Partial`, the spend restriction applies to spells only and not activations.
/// Playing this land resolves its surveil trigger, and activating ability 2 produces restricted mana in `pool.restricted()`.
#[test]
fn gallifrey_council_chamber_surveils_and_produces_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(334, forest())
        .hand(0, &[gallifrey_council_chamber()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, gallifrey_council_chamber()).expect("chamber in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    // The surveil is a *triggered* ability, so it goes on the stack and the
    // question arrives when it resolves, not when the land arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected surveil prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ChoicePrompt::SurveilGraveyard);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, gallifrey_council_chamber(), 2);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.available(ManaColor::Blue), 0);
    let land = on_battlefield(&engine, p0, gallifrey_council_chamber()).expect("land deployed");
    assert!(is_tapped(&engine, land));
}

/// Hellion Crucible: "{T}: Add {C}." / "{1}{R}, {T}: Put a pressure counter on this land..." / "{1}{R}, {T}, Remove two pressure counters..."
/// Under `Coverage::Partial`, pressure counter abilities are omitted because pressure counters have no assigned id.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn hellion_crucible_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(335, forest())
        .hand(0, &[hellion_crucible()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hellion_crucible());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hellion_crucible(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Hive of the Eye Tyrant: "If you control two or more other lands, this land enters tapped." / "{T}: Add {B}." / "{3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature with menace..."
/// Under `Coverage::Partial`, the attack trigger targets an opponent's graveyard card rather than specifically the defending player's graveyard.
/// Playing this land while controlling two or more other lands enters tapped, and paying `{3}{B}` after untapping animates it into a 3/3 Beholder creature with menace.
#[test]
fn hive_of_the_eye_tyrant_animates_into_beholder_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hive_of_the_eye_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hive_of_the_eye_tyrant());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
}

/// Lazotep Quarry: "{T}: Add {C}." / "{T}, Sacrifice a creature: Add one mana of any color." / "{X}{2}, {T}, Sacrifice a Desert..."
/// Under `Coverage::Partial`, the `{X}{2}` reanimation ability is omitted because no filter compares mana values against X.
/// Activating ability 1 sacrifices a creature you control and adds one mana of any chosen color.
#[test]
fn lazotep_quarry_sacrifices_creature_to_produce_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[lazotep_quarry(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    activate(&mut engine, p0, lazotep_quarry(), 1);

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    let quarry = on_battlefield(&engine, p0, lazotep_quarry()).expect("quarry on battlefield");
    assert!(is_tapped(&engine, quarry));
}

/// Memorial to Unity: "This land enters tapped." / "{T}: Add {G}." / "{2}{G}, {T}, Sacrifice this land: Look at the top five cards of your library..."
/// Under `Coverage::Partial`, the sacrifice ability searching the top five cards is omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn memorial_to_unity_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(336, forest())
        .hand(0, &[memorial_to_unity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, memorial_to_unity());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, memorial_to_unity(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Minas Morgul, Dark Fortress: "Minas Morgul enters tapped." / "{T}: Add {B}." / "{3}{B}, {T}: Put a shadow counter on target creature..."
/// Under `Coverage::Partial`, the shadow counter activation is omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn minas_morgul_dark_fortress_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(337, forest())
        .hand(0, &[minas_morgul_dark_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, minas_morgul_dark_fortress());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, minas_morgul_dark_fortress(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// Mirrex: "{T}: Add {C}." / "{T}: Add one mana of any color. Activate only if this land entered this turn." / "{3}, {T}: Create a 1/1 colorless Phyrexian Mite..."
/// Under `Coverage::Partial`, the entered-this-turn any-color ability and token creation are omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn mirrex_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(327, forest()).hand(0, &[mirrex()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrex());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, mirrex(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Mirrorpool: "This land enters tapped." / "{T}: Add {C}." / "{2}{C}, {T}, Sacrifice this land..." / "{4}{C}, {T}, Sacrifice this land..."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its copy abilities are fully realized.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for colorless mana.
#[test]
fn mirrorpool_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(338, forest()).hand(0, &[mirrorpool()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrorpool());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, mirrorpool(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Opal Palace: "{T}: Add {C}." / "{1}, {T}: Add one mana of any color in your commander's color identity..."
/// Under `Coverage::Partial`, the spend rider adding extra +1/+1 counters to a cast commander is omitted.
/// Activating ability 1 with a black commander produces one black mana in the player's pool.
#[test]
fn opal_palace_produces_commander_identity_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(319, forest())
        .commander(0, &[sheoldred_the_apocalypse()])
        .battlefield(0, &[opal_palace(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = on_battlefield(&engine, p0, opal_palace()).expect("palace deployed");
    tap_mana_except(&mut engine, p0, palace);
    activate(&mut engine, p0, opal_palace(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, palace));
}

/// Plaza of Heroes: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell." / "{T}: Add one mana of any color among legendary permanents you control."
/// Under `Coverage::Partial`, the mana ability reading colors among controlled legendary permanents is omitted.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn plaza_of_heroes_produces_restricted_legendary_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(339, forest())
        .battlefield(0, &[plaza_of_heroes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, plaza_of_heroes(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert_eq!(pool.available(ManaColor::White), 0);
    let land = on_battlefield(&engine, p0, plaza_of_heroes()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}

/// Primal Beyond: "As this land enters, you may reveal an Elemental card from your hand..." / "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast an Elemental spell..."
/// Under `Coverage::Partial`, the reveal-from-hand enter clause is omitted, and restricted mana applies to spells only.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn primal_beyond_produces_restricted_elemental_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(328, forest())
        .battlefield(0, &[primal_beyond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, primal_beyond(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert_eq!(pool.available(ManaColor::Red), 0);
    let land = on_battlefield(&engine, p0, primal_beyond()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}

/// Reef Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {U}." / "{1}{U}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn reef_roads_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(324, forest()).hand(0, &[reef_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, reef_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, reef_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Restless Cottage: "This land enters tapped." / "{T}: Add {B} or {G}." / "{2}{B}{G}: This land becomes a 4/4 black and green Horror creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{B}{G}` after untapping animates it into a 4/4 Horror creature.
#[test]
fn restless_cottage_animates_into_horror() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(312, forest())
        .battlefield(0, &[swamp(), swamp(), forest(), forest()])
        .hand(0, &[restless_cottage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_cottage());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_cottage(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (4, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Fortress: "This land enters tapped." / "{T}: Add {W} or {B}." / "{2}{W}{B}: This land becomes a 1/4 white and black Nightmare creature until end of turn. It's still a land."
/// Under `Coverage::Partial`, the attack trigger's life drain targets the opponent seat rather than specifically defending player.
/// Playing this land causes it to enter tapped, and paying `{2}{W}{B}` after untapping animates it into a 1/4 Nightmare creature.
#[test]
fn restless_fortress_animates_into_nightmare() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[plains(), plains(), swamp(), swamp()])
        .hand(0, &[restless_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_fortress());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_fortress(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (1, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Prairie: "This land enters tapped." / "{T}: Add {G} or {W}." / "{2}{G}{W}: This land becomes a 3/3 green and white Llama creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{G}{W}` after untapping animates it into a 3/3 Llama creature.
#[test]
fn restless_prairie_animates_into_llama() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(314, forest())
        .battlefield(0, &[forest(), forest(), plains(), plains()])
        .hand(0, &[restless_prairie()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_prairie());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_prairie(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Ridgeline: "This land enters tapped." / "{T}: Add {R} or {G}." / "{2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{R}{G}` after untapping animates it into a 3/4 Dinosaur creature.
#[test]
fn restless_ridgeline_animates_into_dinosaur() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(315, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[restless_ridgeline()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_ridgeline());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_ridgeline(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Spire: "This land enters tapped." / "{T}: Add {U} or {R}." / "{U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature..."
/// Under `Coverage::Partial`, the granted first strike during your turn is omitted because no modifier grants turn-conditional keywords.
/// Playing this land causes it to enter tapped, and paying `{U}{R}` after untapping animates it into a 2/1 Elemental creature.
#[test]
fn restless_spire_animates_into_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(316, forest())
        .battlefield(0, &[island(), mountain()])
        .hand(0, &[restless_spire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_spire());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_spire(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 1));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Vents: "This land enters tapped." / "{T}: Add {B} or {R}." / "{1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land."
/// Under `Coverage::Partial`, the attack trigger tying discard to draw is omitted.
/// Playing this land causes it to enter tapped, and paying `{1}{B}{R}` after untapping animates it into a 2/3 Insect creature with menace.
#[test]
fn restless_vents_animates_into_insect_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(317, forest())
        .battlefield(0, &[swamp(), mountain(), forest()])
        .hand(0, &[restless_vents()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_vents());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_vents(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
}

/// Restless Vinestalk: "This land enters tapped." / "{T}: Add {G} or {U}." / "{3}{G}{U}: Until end of turn, this land becomes a 5/5 green and blue Plant creature with trample. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{3}{G}{U}` after untapping animates it into a 5/5 Plant creature with trample.
#[test]
fn restless_vinestalk_animates_into_plant_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(318, forest())
        .battlefield(0, &[forest(), forest(), forest(), island(), island()])
        .hand(0, &[restless_vinestalk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_vinestalk());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_vinestalk(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (5, 5));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::TRAMPLE));
    assert!(!is_tapped(&engine, land));
}

/// Rocky Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {R}." / "{1}{R}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for red mana.
#[test]
fn rocky_roads_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(325, forest()).hand(0, &[rocky_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, rocky_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, rocky_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Shifting Woodland: "This land enters tapped unless you control a Forest." / "{T}: Add {G}." / "Delirium — {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn..."
/// Under `Coverage::Partial`, the delirium ability is omitted because no condition counts card types in your graveyard.
/// Controlling a Forest allows Shifting Woodland to enter untapped and immediately tap for green mana.
#[test]
fn shifting_woodland_enters_untapped_with_forest_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[shifting_woodland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shifting_woodland());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, shifting_woodland(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Sunken Palace: "This land enters tapped." / "{T}: Add {U}." / "{1}{U}, {T}, Exile seven cards..."
/// Under `Coverage::Partial`, the graveyard-exile copy activation is omitted because no cost part exiles cards from a graveyard.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn sunken_palace_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(301, forest()).hand(0, &[sunken_palace()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, sunken_palace());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, sunken_palace(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// The Biblioplex: "{T}: Add {C}." / "{2}, {T}: Look at the top card of your library..."
/// Under `Coverage::Partial`, the `{2}, {T}` top-card library ability is omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn the_biblioplex_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(320, forest())
        .hand(0, &[the_biblioplex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_biblioplex());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, the_biblioplex(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Thran Portal: "This land enters tapped unless you control two or fewer other lands." / "As this land enters, choose a basic land type..."
/// Under `Coverage::Partial`, basic land type choice and its mana abilities are omitted.
/// When played while controlling three other lands, Thran Portal enters tapped.
#[test]
fn thran_portal_enters_tapped_with_three_other_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(309, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[thran_portal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, thran_portal());
    assert!(entered_tapped(&engine, land));
}

/// Trenzalore Clocktower: "{T}: Add {U}. Put a time counter on Trenzalore Clocktower." / "{1}{U}, {T}, Remove twelve time counters..."
/// Under `Coverage::Partial`, shuffling the hand into the library in the second ability is omitted.
/// Activating ability 0 adds blue mana to the pool and places a time counter on Trenzalore Clocktower.
#[test]
fn trenzalore_clocktower_taps_for_blue_and_gains_time_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(321, forest())
        .hand(0, &[trenzalore_clocktower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, trenzalore_clocktower());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, trenzalore_clocktower(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(counters_on(&engine, land, CounterKind::Time), 1);
    assert!(is_tapped(&engine, land));
}

/// Wild Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {G}." / "{1}{G}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn wild_roads_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(326, forest()).hand(0, &[wild_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wild_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, wild_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}
