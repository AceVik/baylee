//! `cards/artifacts/mv_6/sword_of_the_ages.rs`, played.
//!
//! Sword of the Ages ({6}, Artifact): "This artifact enters tapped. {T},
//! Sacrifice this artifact and any number of creatures you control: This
//! artifact deals X damage to any target, where X is the total power of the
//! creatures sacrificed this way, then exile this artifact and those creature
//! cards."
//!
//! Written from the printed sentences, the Scryfall ruling (2009-10-01: "The
//! activated ability checks the power the exiled creatures had as they last
//! existed on the battlefield") and the Comprehensive Rules: permanents
//! enter untapped unless something says otherwise (CR 110.5b), "any target"
//! is a creature, player, planeswalker or battle (CR 115.4), a source that
//! would deal 0 damage deals none (CR 120.8), a token in another zone ceases
//! to exist (CR 111.7). The Sword and the creatures are seeded where a test
//! is not about entering; every activation is played through `Engine::apply`
//! and the questions the activation asks are answered as a player answers
//! them.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "201f2434-96b3-408c-a5ce-74d0675920ed"
fn sword_of_the_ages() -> CardIndex {
    card_index("201f2434-96b3-408c-a5ce-74d0675920ed")
}

// {1}{W} instant: "Create two 1/1 white Soldier creature tokens."
fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

// {1}{G} 2/2, no rules text.
fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// What to point the Sword at.
#[derive(Clone, Copy)]
enum Aim {
    Player(PlayerId),
    Creature(ObjectId),
}

/// What the activation asked on its way: the menus a player saw.
#[derive(Default)]
struct Menu {
    /// The objects "any target" offered.
    target_objects: Vec<ObjectId>,
    /// The players "any target" offered.
    target_players: Vec<PlayerId>,
    /// The creatures offered to be sacrificed, when a question listed them.
    sacrifice: Option<Vec<ObjectId>>,
    /// The most creatures the number question allowed, when it asked.
    most: Option<u32>,
}

/// Activates the Sword's one ability as p0 aiming at `aim` and sacrificing
/// `sacrifice` along with it, answering what the game asks in whatever order
/// it asks it (the target, how many, which), and lets the ability resolve.
#[track_caller]
fn fire_the_sword(
    engine: &mut Engine<RegistryLookup>,
    sword: ObjectId,
    aim: Aim,
    sacrifice: &[ObjectId],
) -> Menu {
    let p0 = kit::P0;
    let offered = kit::offered(engine, sword);
    assert_eq!(
        offered.len(),
        1,
        "the Sword prints one activated ability and it is on offer: {offered:?}"
    );
    kit::activate_object(engine, p0, sword, offered[0]);
    let mut menu = Menu::default();
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                options,
                player_options,
                ..
            } => {
                menu.target_objects = options;
                menu.target_players = player_options;
                let (objects, players) = match aim {
                    Aim::Player(p) => (vec![], vec![p]),
                    Aim::Creature(c) => (vec![c], vec![]),
                };
                engine
                    .apply(p0, PlayerAction::ChooseTargets { objects, players })
                    .expect("the target came out of the menu");
            }
            Pending::ChooseNumber { min, max, .. } => {
                menu.most = Some(max);
                let n = u32::try_from(sacrifice.len()).expect("a handful");
                assert!(
                    (min..=max).contains(&n),
                    "sacrificing {n} creatures is within what was asked ({min}..={max})"
                );
                engine
                    .apply(p0, PlayerAction::ChooseNumber(n))
                    .expect("the number");
            }
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                let picks: Vec<ObjectId> = options
                    .iter()
                    .copied()
                    .filter(|o| sacrifice.contains(o))
                    .take(usize::from(max))
                    .collect();
                assert_eq!(
                    picks.len(),
                    sacrifice.len(),
                    "every creature the test sacrifices is on the menu {options:?}"
                );
                assert!(
                    picks.len() >= usize::from(min),
                    "the answer is at least the {min} the question needs"
                );
                menu.sacrifice = Some(options);
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: picks })
                    .expect("the creatures");
            }
            Pending::Priority { .. } if !stack_is_empty(engine) => {
                let Pending::Priority { player, .. } = engine.pending().clone() else {
                    unreachable!()
                };
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing resolves the ability");
            }
            Pending::Priority { .. } => return menu,
            other => panic!("nothing in this test answers {other:?}"),
        }
    }
    panic!(
        "the activation never settled; still at {:?}",
        engine.pending()
    );
}

/// A game with the Sword (untapped, seeded) on p0's side and the creatures
/// named on each side; p0 holds priority in the first main phase.
fn sword_game(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    hand: &[CardIndex],
    lands: usize,
) -> Engine<RegistryLookup> {
    let mut p0_board = vec![sword_of_the_ages()];
    p0_board.extend_from_slice(mine);
    p0_board.extend(std::iter::repeat_n(forest(), lands));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &p0_board)
        .battlefield(1, theirs)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    engine
}

fn exile_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Exile(seat)).clone()
}

/// "This artifact enters tapped." Cast for {6}, the Sword arrives tapped, and
/// the ability, whose cost is {T}, is not on offer; it untaps in its
/// controller's next untap step (an artifact that is not a creature has no
/// summoning sickness to wait out) and is then on offer. The control is a
/// Sol Ring cast the same way, which enters untapped (CR 110.5b).
#[test]
fn sword_of_the_ages_enters_tapped_and_is_usable_from_the_next_turn() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![forest(); 6])
        .hand(0, &[sword_of_the_ages()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, sword_of_the_ages());
    pass_until(&mut engine, stack_is_empty);
    let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword entered");
    assert!(is_tapped(&engine, sword), "it entered tapped");
    assert!(
        kit::offered(&engine, sword).is_empty(),
        "tapped: {{T}} cannot be paid, so the ability is not on offer"
    );

    kit::walk_to_next_own_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, sword),
        "it untapped in p0's next untap step"
    );
    assert_eq!(
        kit::offered(&engine, sword).len(),
        1,
        "untapped: its ability is on offer"
    );

    // The control: an artifact that does not say it enters tapped doesn't.
    let mut control = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut control);
    reach_main_phase(&mut control, p0);
    cast_from_hand(&mut control, p0, sol_ring());
    pass_until(&mut control, stack_is_empty);
    let ring = on_battlefield(&control, p0, sol_ring()).expect("the Sol Ring entered");
    assert!(!is_tapped(&control, ring), "a Sol Ring enters untapped");
}

/// The core sentence: sacrifice the Sword and two creatures (a 2/2 and a 3/3)
/// and the Sword deals 5 — their total power — to the target player, "then
/// exile this artifact and those creature cards": all three end in exile and
/// none in the graveyard. A third creature that was not sacrificed stays,
/// the opponent's creature is not on the menu of creatures to sacrifice, and
/// the damage is the Sword's own (a source), dealt once.
#[test]
fn sword_of_the_ages_deals_the_total_power_of_the_sacrificed_creatures_then_exiles_them() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = sword_game(
        &[gray_ogre(), hill_giant(), grizzly_bears()],
        &[hill_giant()],
        &[],
        0,
    );
    let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("my Giant");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let their_giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    let journal_before = engine.journal().entries().len();

    let menu = fire_the_sword(&mut engine, sword, Aim::Player(p1), &[ogre, giant]);

    let mut offered = menu
        .sacrifice
        .expect("the creatures to sacrifice were offered");
    offered.sort_unstable();
    let mut mine = vec![ogre, giant, bears];
    mine.sort_unstable();
    assert_eq!(
        offered, mine,
        "the creatures p0 controls, all of them, and neither the Sword nor p1's Giant"
    );
    assert!(
        menu.target_players.contains(&p0) && menu.target_players.contains(&p1),
        "\"any target\": both players are on the menu, got {:?}",
        menu.target_players
    );
    assert!(
        menu.target_objects.contains(&their_giant),
        "\"any target\": a creature is on the menu too"
    );

    assert_eq!(
        kit::life(&engine, p1),
        15,
        "2 + 3 damage to the target player"
    );
    assert_eq!(kit::life(&engine, p0), 20, "and none to anyone else");
    let hits: Vec<_> = damage_events(&engine, journal_before)
        .into_iter()
        .filter(|(_, _, amount, _)| *amount > 0)
        .collect();
    assert_eq!(hits.len(), 1, "one damage event, got {hits:?}");
    let (source, target, amount, combat) = hits[0];
    assert_eq!(source, sword, "the Sword is the source of the damage");
    assert!(matches!(target, crate::event::DamageTarget::Player(p) if p == p1));
    assert_eq!((amount, combat), (5, false), "5 damage, not combat damage");

    for (id, what) in [
        (sword, "the Sword"),
        (ogre, "the Ogre"),
        (giant, "my Giant"),
    ] {
        assert_eq!(
            kit::zone_of(&engine, id),
            Zone::Exile,
            "{what} was exiled, not left in the graveyard"
        );
    }
    let mut exiled = exile_of(&engine, p0);
    exiled.sort_unstable();
    let mut expected = vec![sword, ogre, giant];
    expected.sort_unstable();
    assert_eq!(
        exiled, expected,
        "exactly those three, in their owner's exile"
    );
    assert!(
        kit::graveyard_of(&engine, p0).is_empty(),
        "nothing remains in p0's graveyard"
    );
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Battlefield,
        "a creature that was not sacrificed stays"
    );
    assert_eq!(
        kit::zone_of(&engine, their_giant),
        Zone::Battlefield,
        "so does the opponent's"
    );
}

/// "Any number" includes none: sacrificing only the Sword makes X = 0, so
/// nothing is dealt (CR 120.8) — and the Sword is still sacrificed and
/// exiled, because that part is not "any number". A creature that could have
/// been sacrificed is right there and is left alone.
#[test]
fn sword_of_the_ages_with_no_creatures_deals_nothing_and_exiles_only_itself() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = sword_game(&[hill_giant()], &[], &[], 0);
    let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("my Giant");
    let journal_before = engine.journal().entries().len();

    fire_the_sword(&mut engine, sword, Aim::Player(p1), &[]);

    assert_eq!(kit::life(&engine, p1), 20, "X is 0: no damage");
    assert_eq!(kit::life(&engine, p0), 20);
    assert!(
        damage_events(&engine, journal_before)
            .iter()
            .all(|(_, _, amount, _)| *amount == 0),
        "no damage is dealt at all"
    );
    assert_eq!(
        kit::zone_of(&engine, sword),
        Zone::Exile,
        "the Sword is exiled"
    );
    assert_eq!(exile_of(&engine, p0), vec![sword], "and only the Sword");
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Battlefield,
        "no creature was sacrificed, so none is exiled"
    );
}

/// "As they last existed": a creature pumped by Giant Growth ({G}, +3/+3
/// until end of turn) counts its power with the pump, 2 + 3 = 5, where the
/// same Ogre unpumped (the control) counts 2.
#[test]
fn sword_of_the_ages_counts_the_power_a_sacrificed_creature_had_with_its_pump() {
    let (p0, p1) = (kit::P0, kit::P1);
    for (pumped, damage) in [(true, 5), (false, 2)] {
        let hand: &[CardIndex] = if pumped { &[giant_growth()] } else { &[] };
        let mut engine = sword_game(&[gray_ogre()], &[hill_giant()], hand, 1);
        let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword");
        let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
        if pumped {
            cast_from_hand(&mut engine, p0, giant_growth());
            let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
                panic!(
                    "Giant Growth asks for its target, got {:?}",
                    engine.pending()
                )
            };
            assert!(
                options.len() >= 2 && options.contains(&ogre),
                "the Ogre is one of at least two creatures to pick from: {options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![ogre],
                    },
                )
                .expect("Giant Growth targets the Ogre");
            pass_until(&mut engine, stack_is_empty);
        }
        assert_eq!(
            pt(&engine, ogre),
            if pumped { (5, 5) } else { (2, 2) },
            "the Ogre as it stands before the sacrifice"
        );

        fire_the_sword(&mut engine, sword, Aim::Player(p1), &[ogre]);

        assert_eq!(
            kit::life(&engine, p1),
            20 - damage,
            "pumped: {pumped}: the power it had on the battlefield"
        );
        assert_eq!(kit::zone_of(&engine, ogre), Zone::Exile);
        assert_eq!(kit::zone_of(&engine, sword), Zone::Exile);
    }
}

/// "Any target" includes a creature, and X is exact: the Sword sacrificing a
/// Hill Giant (power 3) points at the opponent's Hill Giant (toughness 3) and
/// kills it; sacrificing a Gray Ogre (power 2) instead (the control) leaves
/// the same target alive with exactly 2 damage on it.
#[test]
fn sword_of_the_ages_can_kill_a_creature_exactly_when_the_power_reaches_its_toughness() {
    let (p0, p1) = (kit::P0, kit::P1);
    for (sacrificed, dies) in [(hill_giant(), true), (gray_ogre(), false)] {
        let mut engine = sword_game(&[sacrificed], &[hill_giant()], &[], 0);
        let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword");
        let mine = on_battlefield(&engine, p0, sacrificed).expect("the creature to sacrifice");
        let target = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");

        fire_the_sword(&mut engine, sword, Aim::Creature(target), &[mine]);

        assert_eq!(
            kit::life(&engine, p1),
            20,
            "the creature was the target, not the player"
        );
        if dies {
            assert!(
                on_battlefield(&engine, p1, hill_giant()).is_none()
                    && in_graveyard(&engine, p1, hill_giant()).is_some(),
                "3 damage kills a 3/3"
            );
        } else {
            assert_eq!(
                kit::zone_of(&engine, target),
                Zone::Battlefield,
                "2 damage does not kill a 3/3"
            );
            assert_eq!(kit::damage_on(&engine, target), 2, "but it is marked");
        }
        assert_eq!(kit::zone_of(&engine, mine), Zone::Exile);
    }
}

/// A sacrificed token counts its power and then simply stops existing
/// (CR 111.7): it is not among the "creature cards" in exile. A Soldier token
/// (power 1) and a Gray Ogre (power 2) make 3; the Ogre card and the Sword
/// are in exile and nothing else is; the second Soldier is untouched.
#[test]
fn sword_of_the_ages_counts_a_sacrificed_token_and_exiles_only_cards() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sword_of_the_ages(), gray_ogre(), plains(), forest()])
        .hand(0, &[raise_the_alarm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "two Soldiers");
    let sword = on_battlefield(&engine, p0, sword_of_the_ages()).expect("the Sword");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");

    fire_the_sword(&mut engine, sword, Aim::Player(p1), &[tokens[0], ogre]);

    assert_eq!(kit::life(&engine, p1), 17, "1 (the token) + 2 (the Ogre)");
    let mut exiled = exile_of(&engine, p0);
    exiled.sort_unstable();
    let mut expected = vec![sword, ogre];
    expected.sort_unstable();
    assert_eq!(
        exiled, expected,
        "the Sword and the Ogre card are exiled; the token is not a card and is nowhere"
    );
    assert!(
        engine
            .state()
            .object(tokens[0])
            .is_none_or(|o| o.zone != Zone::Battlefield && o.zone != Zone::Exile),
        "the sacrificed token is not on the battlefield and not in exile"
    );
    assert_eq!(
        kit::zone_of(&engine, tokens[1]),
        Zone::Battlefield,
        "the other Soldier was not sacrificed"
    );
}
