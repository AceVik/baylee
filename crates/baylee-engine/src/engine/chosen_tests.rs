//! Effects where **each player chooses among their own permanents**.
//!
//! Three effects share one shape: the effect names a filter and a set of
//! players, and every one of those players is asked, in turn, to pick one of
//! their own permanents matching it. What happens to the pick is the only
//! difference — [`Effect::SacrificeFilter`] sends it to the graveyard,
//! [`Effect::DestroyChosenForPlayers`] destroys it.
//!
//! What matters about the shape, and what these tests exist to pin, is that
//! **the choice belongs to the permanent's controller and not to the
//! spell's**. An edict that let the caster pick would be a targeted removal
//! spell wearing an edict's text, and nothing in the type system says
//! otherwise: the chain hands `Pending::ChooseCards` a `player`, and a wrong
//! one compiles.
//!
//! The options are also a *filter over that player's own battlefield*, so a
//! mode naming nontoken creatures must not offer a token — which is the
//! second thing a wrong implementation gets away with silently, because a
//! board with one creature on it agrees with every filter.

use super::testkit::{
    Duel, RegistryLookup, SEED, card_index, cast_from_hand, keep_mulligans, quiet_creature,
    reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_core::ids::CardIndex;

fn sheoldreds_edict() -> CardIndex {
    card_index("217062f5-96f1-454c-9507-17f34ef37070")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Casts the edict from seat 0 and answers the mode question with `mode`.
fn cast_edict(mode: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[sheoldreds_edict()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    reach_main_phase(&mut engine, seat);
    cast_from_hand(&mut engine, seat, sheoldreds_edict());

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a modal spell asks its mode on the way to the stack (CR 601.2b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 3, "the edict prints three modes");
    engine
        .apply(seat, PlayerAction::ChooseMode(mode))
        .expect("naming a printed mode is legal");
    engine
}

#[test]
fn an_edict_is_answered_by_the_creature_s_controller_and_not_the_caster() {
    let mut engine = cast_edict(0);
    let them = PlayerId::new(1);

    // Pass priority until the spell resolves and the question is asked.
    let seat = PlayerId::new(0);
    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let asked = match engine.pending() {
            Pending::Priority { player, .. } => *player,
            other => panic!("expected priority or the edict's question, got {other:?}"),
        };
        engine
            .apply(asked, PlayerAction::PassPriority)
            .expect("passing is always legal");
        let _ = seat;
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
            "the edict asks somebody to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, them,
        "an edict is not removal: the opponent picks, the caster does not"
    );
    assert_eq!(options.len(), 2, "both of their creatures may be given up");
    assert_eq!(
        (min, max),
        (1, 1),
        "'sacrifices a creature' is not optional"
    );

    let victim = options[0];
    let survivor = options[1];
    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("naming one of the offered creatures is legal");

    assert_eq!(
        engine.state().object(victim).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "the chosen creature was not sacrificed"
    );
    assert_eq!(
        engine.state().object(survivor).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "an edict takes one creature, not the board"
    );
}

#[test]
fn the_token_mode_offers_no_card_and_therefore_asks_nobody() {
    // Mode two is "a creature token of their choice", and seat one's two
    // creatures are cards. A filter that matched anyway would be invisible on
    // a board where every creature is legal, which is why the discriminating
    // board has *only* the wrong kind on it.
    let mut engine = cast_edict(1);
    let seat = PlayerId::new(0);
    let them = PlayerId::new(1);

    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => panic!(
                "no token is on the battlefield, yet {} permanent(s) were offered",
                options.len()
            ),
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is legal");
            }
            other => panic!("unexpected question: {other:?}"),
        }
        if stack_is_empty(&engine) {
            break;
        }
    }

    assert!(stack_is_empty(&engine), "the edict never left the stack");
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(them))
            .is_empty(),
        "nothing of theirs was sacrificed, so their graveyard stays empty"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(seat))
            .len(),
        1,
        "the edict itself goes to its own owner's graveyard, and only it"
    );
}

fn sheoldred() -> CardIndex {
    card_index("97652492-7906-4d79-983c-fa1dc1239eba")
}

fn taoist_hermit() -> CardIndex {
    card_index("960b1251-12af-4766-8df1-bb98e7e967f5")
}

/// Chapter I of The True Scriptures: "For each opponent, destroy up to one
/// target creature or planeswalker that player controls."
///
/// The road to it is part of the test: Sheoldred is a creature until
/// `{4}{B}` exiles her and returns her transformed, and that ability is
/// offered only while an opponent holds eight cards in their graveyard.
///
/// Three seats, because a duel cannot tell "one per opponent" from "one":
/// the Saga's controller is asked once for each opponent, each question
/// offers only that player's permanents, a hexproof creature is not on the
/// menu (it is targeting), and both answers die together.
#[test]
#[allow(clippy::too_many_lines)] // the road to the back face is part of the test
fn a_saga_chapter_targets_one_permanent_of_each_opponent() {
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(
            0,
            &[sheoldred(), swamp(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[quiet_creature(), quiet_creature(), taoist_hermit()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let seat = PlayerId::new(0);
    let (left, right) = (PlayerId::new(1), PlayerId::new(2));
    super::testkit::seed_graveyard(&mut engine, left, 8);
    reach_main_phase(&mut engine, seat);

    // Tap for {4}{B} and press the transform ability.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a swamp taps for black");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after tapping, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .next()
        .expect("eight cards in their graveyard make the transform legal");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the transform ability activates at sorcery speed");

    let controller_of =
        |e: &Engine<RegistryLookup>, id: ObjectId| e.state().object(id).map(|o| o.controller);
    let mut asked = Vec::new();
    let mut doomed = Vec::new();
    for _ in 0..80 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, seat, "the Saga's controller chooses the targets");
                assert_eq!((min, max), (0, 1), "up to one per opponent");
                let owners: Vec<_> = options.iter().map(|o| controller_of(&engine, *o)).collect();
                assert!(
                    owners.windows(2).all(|w| w[0] == w[1]),
                    "one question offers one opponent's permanents: {owners:?}"
                );
                asked.push(owners[0].expect("a permanent"));
                let pick = options[0];
                doomed.push(pick);
                if owners[0] == Some(left) {
                    assert_eq!(
                        options.len(),
                        2,
                        "the two Elves and not the hexproof Hermit"
                    );
                }
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseTargets {
                            objects: vec![pick],
                            players: vec![],
                        },
                    )
                    .expect("an offered permanent");
            }
            Pending::Priority { player, .. } => {
                if asked.len() == 2 && stack_is_empty(&engine) {
                    break;
                }
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is legal");
            }
            other => panic!("unexpected question on the way through chapter I: {other:?}"),
        }
    }
    assert_eq!(
        asked,
        vec![left, right],
        "each opponent once, in turn order"
    );
    for id in doomed {
        assert_eq!(
            engine.state().object(id).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard),
            "a chosen target was not destroyed"
        );
    }
}

fn innocent_blood() -> CardIndex {
    card_index("6791ec3c-c397-4087-8c8c-84d3797df415")
}

fn blood_artist() -> CardIndex {
    card_index("310f141c-7f37-4729-aed6-dd9c09db448d")
}

fn curfew() -> CardIndex {
    card_index("6ff1e793-1569-4887-b6bb-d9855c5e8a26")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn pass_to_card_choice<L: crate::state::CardLookup>(engine: &mut Engine<L>) {
    for _ in 0..16 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority on the way to card choice, got {:?}",
                engine.pending()
            );
        };
        engine
            .apply(player, PlayerAction::PassPriority)
            .expect("passing priority is legal");
    }
    panic!("never reached ChooseCards: {:?}", engine.pending());
}

fn drain_triggers_targeting<L: crate::state::CardLookup>(
    engine: &mut Engine<L>,
    target_player: PlayerId,
) {
    for _ in 0..32 {
        if stack_is_empty_any(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            return;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![target_player],
                        },
                    )
                    .expect("targeting opponent is legal");
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("keeping default order is legal");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is legal");
            }
            other => panic!("unexpected pending while draining triggers: {other:?}"),
        }
    }
    panic!("triggers never drained: {:?}", engine.pending());
}

fn stack_is_empty_any<L: crate::state::CardLookup>(engine: &Engine<L>) -> bool {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .is_empty()
}

fn setup_apnap_innocent_blood() -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, swamp(), 3)
        .hand(1, &[innocent_blood()])
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[swamp(), quiet_creature(), quiet_creature()])
        .battlefield(2, &[quiet_creature()])
        .start();
    let p1 = PlayerId::new(1);
    assert!(super::testkit::walk_to_own_main(&mut engine, p1));
    assert_eq!(engine.state().turn.active, p1);
    cast_from_hand(&mut engine, p1, innocent_blood());
    pass_to_card_choice(&mut engine);
    engine
}

#[test]
fn each_player_sacrifice_asks_in_apnap_order_and_holds_until_all_choose() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = setup_apnap_innocent_blood();
    let mut alt = setup_apnap_innocent_blood();

    let Pending::ChooseCards {
        player: first_asked,
        options: p1_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(first_asked, p1, "CR 101.4: active player chooses first");
    assert_eq!(p1_opts.len(), 2);

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_opts[0]],
            },
        )
        .expect("p1 picks first creature");
    alt.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![p1_opts[1]],
        },
    )
    .expect("alt p1 picks second creature");
    assert_ne!(
        engine.snapshot_hash(),
        alt.snapshot_hash(),
        "held sacrifice choice must distinguish search/replay hashes before creatures move"
    );
    assert_eq!(
        engine.state().object(p1_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "p1's chosen creature stays on the battlefield until every player chooses"
    );
    let held = engine
        .resolution
        .as_ref()
        .and_then(|r| r.awaiting.as_ref())
        .expect("awaiting SacrificeFilter")
        .held_choices();
    assert_eq!(held.len(), 1);
    assert_eq!((held[0].0, held[0].1.object), (p1, p1_opts[0]));

    let Pending::ChooseCards {
        player: second_asked,
        options: p2_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p2 ChooseCards");
    };
    assert_eq!(second_asked, p2, "CR 101.4: seat 2 follows active seat 1");
    engine
        .apply(
            p2,
            PlayerAction::ChooseObjects {
                objects: vec![p2_opts[0]],
            },
        )
        .expect("p2 picks");
    assert_eq!(
        engine.state().object(p2_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "p2's chosen creature also stays on the battlefield until p0 chooses"
    );

    let Pending::ChooseCards {
        player: third_asked,
        options: p0_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 ChooseCards");
    };
    assert_eq!(third_asked, p0, "CR 101.4: seat 0 wraps around last");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_opts[0]],
            },
        )
        .expect("p0 picks");

    for victim in [p1_opts[0], p2_opts[0], p0_opts[0]] {
        assert_eq!(
            engine.state().object(victim).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard),
            "every held choice is sacrificed together once all have answered"
        );
    }
}

#[test]
fn simultaneous_sacrifice_lets_departing_blood_artist_see_later_apnap_sacrifices() {
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[innocent_blood()])
        .battlefield(0, &[swamp(), blood_artist()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, innocent_blood());
    pass_to_card_choice(&mut engine);

    let Pending::ChooseCards {
        player: first,
        options: p0_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 sacrifice prompt");
    };
    assert_eq!(first, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_opts[0]],
            },
        )
        .expect("p0 chooses Blood Artist");

    let Pending::ChooseCards {
        player: second,
        options: p1_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 sacrifice prompt");
    };
    assert_eq!(second, p1);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_opts[0]],
            },
        )
        .expect("p1 chooses their creature");

    drain_triggers_targeting(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "CR 603.10a: Blood Artist dies simultaneously with p1's creature and triggers twice"
    );
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn return_chosen_to_hand_asks_in_apnap_order_and_holds_until_all_choose() {
    let mut engine = Duel::table(SEED, island(), 3)
        .hand(1, &[curfew()])
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[island(), quiet_creature()])
        .battlefield(2, &[quiet_creature()])
        .start();
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    assert!(super::testkit::walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, curfew());
    pass_to_card_choice(&mut engine);

    let mut asked = Vec::new();
    let mut picks = Vec::new();
    for expected in [p1, p2, p0] {
        for &earlier in &picks {
            assert_eq!(
                engine.state().object(earlier).map(|o| o.zone),
                Some(crate::zone::Zone::Battlefield),
                "earlier Curfew choice must stay on battlefield until all choose"
            );
        }
        let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        else {
            panic!("expected Curfew ChooseCards, got {:?}", engine.pending());
        };
        assert_eq!(player, expected);
        asked.push(player);
        picks.push(options[0]);
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("choosing controlled creature is legal");
    }
    assert_eq!(asked, vec![p1, p2, p0]);
    for (seat, returned) in [(p1, picks[0]), (p2, picks[1]), (p0, picks[2])] {
        assert!(
            engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Hand(seat))
                .contains(&returned),
            "chosen permanent returns to its owner's hand once all have chosen"
        );
    }
}

#[test]
fn return_chosen_to_hand_lets_departing_watcher_see_later_apnap_returns() {
    use baylee_cards_dsl::{
        CardDef, CommanderRule, Cost, Coverage, Effect, Filter, KeywordSet, PartnerKind, PlayerRel,
        Trigger, activated, triggered,
    };

    static THIS_OR_ANOTHER_CREATURE: Filter = Filter::Or(&[
        Filter::This,
        Filter::And(&[Filter::CREATURE, Filter::Another]),
    ]);
    static GAIN_ONE_LIFE: [Effect; 1] = [Effect::gain_life(1)];
    let bounce_shrine = synthetic::land(
        910_001,
        "Curfew Bell",
        Box::leak(Box::new([activated!(
            Cost::TAP,
            &[Effect::ReturnChosenToHand {
                who: PlayerRel::EachPlayer,
                filter: &Filter::CREATURE,
            }]
        )])),
    );
    let witness = Box::leak(Box::new(CardDef {
        index: CardIndex::new(910_002),
        oracle_id: "test-witness",
        scryfall_id: "test-witness",
        faces: Box::leak(Box::new([synthetic::creature_face("Witness", 1, 1, &[])])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities: Box::leak(Box::new([triggered!(
            Trigger::LeavesBattlefield(&THIS_OR_ANOTHER_CREATURE),
            &GAIN_ONE_LIFE
        )])),
    }));

    let preset = synthetic::preset_both(SEED, &[910_001, 910_002], &[quiet_creature().get()]);
    let lookup = synthetic::SyntheticLookup::new(vec![bounce_shrine, witness]);
    let mut engine = Engine::new(&preset, lookup).expect("synthetic game starts");
    synthetic::keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let shrine = synthetic::permanents(&engine, 910_001)[0];
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shrine,
                ability_index: 0,
            },
        )
        .expect("tap Curfew Bell");
    pass_to_card_choice(&mut engine);

    for expected in [p0, p1] {
        let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        else {
            panic!("expected ReturnChosenToHand prompt");
        };
        assert_eq!(player, expected);
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("choose creature to return");
    }

    drain_triggers_targeting(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "CR 603.10a: departing Witness sees p1's creature leave in the same simultaneous batch"
    );
}

fn destroy_chosen_altar(index: u32) -> &'static baylee_cards_dsl::CardDef {
    use baylee_cards_dsl::{Cost, Effect, Filter, PlayerRel, activated};

    synthetic::land(
        index,
        "Altar of Ruin",
        Box::leak(Box::new([activated!(
            Cost::TAP,
            &[Effect::DestroyChosenForPlayers {
                who: PlayerRel::EachPlayer,
                filter: &Filter::CREATURE,
            }]
        )])),
    )
}

#[test]
fn destroy_chosen_for_players_asks_in_apnap_order_and_destroys_simultaneously() {
    let shrine = destroy_chosen_altar(910_003);
    let mut preset = synthetic::preset_both(
        SEED,
        &[quiet_creature().get()],
        &[910_003, quiet_creature().get()],
    );
    let mut third = preset.seats[0].clone();
    third.starting_battlefield = preset.seats[0].starting_battlefield.clone();
    preset.seats.push(third);
    preset.house_rules.starting_player = Some(1);

    let lookup = synthetic::SyntheticLookup::new(vec![shrine]);
    let mut engine = Engine::new(&preset, lookup).expect("3-seat synthetic game starts");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    assert_eq!(engine.state().turn.active, p1);

    let altar = synthetic::permanents(&engine, 910_003)[0];
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .expect("activate Altar of Ruin");
    pass_to_card_choice(&mut engine);

    let Pending::ChooseCards {
        player,
        options: p1_opts,
        min,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 DestroyChosen");
    };
    assert_eq!((player, min), (p1, 0));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_opts[0]],
            },
        )
        .expect("p1 chooses their creature");
    assert_eq!(
        engine.state().object(p1_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "p1's pick stays on battlefield while p2 and p0 decide"
    );

    let Pending::ChooseCards {
        player,
        options: p2_opts,
        min,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p2 DestroyChosen");
    };
    assert_eq!((player, min), (p2, 0));
    engine
        .apply(p2, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("min 0 lets p2 decline");

    let Pending::ChooseCards {
        player,
        options: p0_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 DestroyChosen");
    };
    assert_eq!(player, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_opts[0]],
            },
        )
        .expect("p0 chooses their creature");

    assert_eq!(
        engine.state().object(p1_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(p0_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(p2_opts[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield)
    );
}

#[test]
fn destroy_chosen_for_players_lets_departing_blood_artist_see_all_deaths() {
    let shrine = destroy_chosen_altar(910_004);
    let preset = synthetic::preset_both(
        SEED,
        &[910_004, blood_artist().get()],
        &[quiet_creature().get()],
    );
    let lookup = synthetic::SyntheticLookup::new(vec![shrine]);
    let mut engine = Engine::new(&preset, lookup).expect("synthetic game starts");
    synthetic::keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let altar = synthetic::permanents(&engine, 910_004)[0];
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .expect("activate Altar of Ruin");
    pass_to_card_choice(&mut engine);

    for expected in [p0, p1] {
        let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        else {
            panic!("expected DestroyChosen prompt");
        };
        assert_eq!(player, expected);
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("choose creature to destroy");
    }

    drain_triggers_targeting(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "CR 603.10a: Blood Artist destroyed in p0's slot sees p1's simultaneous destruction"
    );
    assert_eq!(engine.state().players[1].life, 18);
}
