//! Beat 18 / Engine Wave 13a deterministic live-play rule tests:
//! 1. *Fireglass Mentor* & *You Find Some Prisoners*: `ChooseZone::Found` over
//!    exiled top cards + `Effect::ChooseCardEx` + `ExileWithPermission` play/cast
//!    without spurious `Exile -> Exile` re-entry events.
//! 2. *Queen Kayla*-style `ChooseZone::Hand` choose + `reveal_chosen` vs private
//!    non-reveal hidden-info invariant, `for_unchosen` bindings, and `CR 608.2d` /
//!    `CR 609.3` option clamping.
//! 3. *Killing Wave* per-player chooser scope (`Filter::ControlledBy(PlayerRel::EventPlayer)`
//!    and `Filter::ControlledByChooser`) across 3 seats in APNAP turn order.
//! 4. *Liliana of the Veil* `-6` battlefield `Effect::SeparateIntoTwoPiles` (`CR 700.3`)
//!    with opponent pile selection and sacrifice (including empty-pile choice).
//! 5. *Jace, Architect of Thought* `-2` and *Hostile Negotiations* `only_found: true`
//!    `Effect::SeparateIntoTwoPiles` (including `BottomOfLibraryChosenOrder` arrange,
//!    face-down exile `TurnFaceUp`, and empty-pile edge cases in both directions).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::ChoicePrompt;
use crate::event::{Cause, GameEvent};
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    Amount, ChooseMove, ChooseZone, ExilePermission, Filter, PlayWindow, PlayerRel, RepeatDomain,
    RevealUntilFound, RevealUntilRest, TargetReq, TargetSpec, ZoneSel, spell,
};

const MENTOR_PROBE: u32 = 99_940;
const EXILED_BEAR: u32 = 99_941;
const EXILED_SPARK: u32 = 99_942;
const KAYLA_PROBE: u32 = 99_943;
const PRIVATE_PROBE: u32 = 99_944;
const RELIC_ONE: u32 = 99_945;
const KNIGHT_TWO: u32 = 99_946;
const GIANT_FIVE: u32 = 99_947;
const KILLING_WAVE_PROBE: u32 = 99_948;
const CHOOSER_SCOPE_PROBE: u32 = 99_949;
const LILIANA_ULT_PROBE: u32 = 99_950;
const JACE_PILES_PROBE: u32 = 99_951;
const NEGOTIATIONS_PROBE: u32 = 99_952;
const TOKEN_A: u32 = 99_953;
const TOKEN_B: u32 = 99_954;
const TOKEN_C: u32 = 99_955;

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn put_hand_card_on_library_top(engine: &mut Bench, owner: PlayerId, index: u32) -> ObjectId {
    let id = *objects(engine, ZoneLocation::Hand(owner), index)
        .first()
        .expect("card in hand");
    let _ = engine.state.move_object(
        id,
        ZoneLocation::Library(owner),
        ZonePosition::Top,
        Cause::Effect,
    );
    id
}

fn resolve_top_of_stack(engine: &mut Bench) {
    let seat_count = engine.state().players.len();
    for _ in 0..seat_count {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            break;
        };
        engine
            .apply(player, PlayerAction::PassPriority)
            .expect("pass priority");
    }
}

// -----------------------------------------------------------------------------
// 1. Fireglass Mentor: exiled `ChooseZone::Found` choose + play this turn
// -----------------------------------------------------------------------------

#[test]
fn fireglass_mentor_found_exile_choice_grants_play_only_to_chosen_card() {
    static MENTOR_EFFECTS: &[Effect] = &[Effect::RevealUntilThen {
        library: PlayerRel::You,
        filter: &Filter::Any,
        count: Amount::Fixed(2),
        max_revealed: Some(Amount::Fixed(2)),
        exile_while_revealing: true,
        found: RevealUntilFound::Exile,
        rest: RevealUntilRest::Exile,
        then: &[Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone: ChooseZone::Found,
            filter: &Filter::Any,
            min: 1,
            max: Amount::Fixed(1),
            reveal_chosen: false,
            movement: ChooseMove::None,
            not_chosen_movement: ChooseMove::None,
            then: &[Effect::ExileWithPermission {
                target: TargetSpec::EventObject,
                until: None,
                permission: ExilePermission::play(PlayWindow::ThisTurn),
            }],
            for_unchosen: &[],
        }],
    }];
    static MENTOR_ABILS: &[AbilityDef] = &[free(MENTOR_EFFECTS, None)];
    static SPARK_ABILS: &[AbilityDef] = &[spell!(&[Effect::gain_life(3)])];

    let cards = vec![
        card(
            MENTOR_PROBE,
            face("Mentor Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            MENTOR_ABILS,
        ),
        card(
            EXILED_BEAR,
            creature_face("Exiled Bear", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            EXILED_SPARK,
            face("Exiled Spark", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            SPARK_ABILS,
        ),
    ];

    let mut engine = bench(
        1301,
        cards,
        [
            Seat::with(&[MENTOR_PROBE]).holding(&[EXILED_BEAR, EXILED_SPARK]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bear = put_hand_card_on_library_top(&mut engine, me(), EXILED_BEAR);
    let spark = put_hand_card_on_library_top(&mut engine, me(), EXILED_SPARK);
    refresh_offer(&mut engine);

    let probe = objects(&engine, ZoneLocation::Battlefield, MENTOR_PROBE)[0];
    let journal_before = engine.state().journal.entries().len();
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate Mentor Probe");
    resolve_top_of_stack(&mut engine);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards over Found exiled cards, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, me());
    assert_eq!(min, 1);
    assert_eq!(max, 1);
    assert_eq!(options, vec![spark, bear]);

    let spark_ver_before = engine
        .state()
        .object(spark)
        .expect("spark in exile")
        .version;
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![spark],
            },
        )
        .expect("choose Exiled Spark");

    // Both cards remain in exile, and granting permission to an already-exiled
    // card must not emit a bogus Exile -> Exile ZoneChanged event or bump version.
    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(spark).unwrap().version,
        spark_ver_before
    );
    let exile_to_exile = engine.state().journal.entries()[journal_before..]
        .iter()
        .any(|e| {
            matches!(
                e.event,
                GameEvent::ZoneChanged {
                    from: Zone::Exile,
                    to: Zone::Exile,
                    ..
                }
            )
        });
    assert!(
        !exile_to_exile,
        "ExileWithPermission on an already-exiled Found card must not re-move Exile -> Exile"
    );

    // Only the chosen card (`spark`) is castable from exile; `bear` was not chosen.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected Priority after Mentor resolution");
    };
    assert!(
        legal.castable.contains(&spark),
        "chosen exiled card must be castable this turn"
    );
    assert!(
        !legal.castable.contains(&bear),
        "unchosen exiled card must NOT gain play permission"
    );

    // Cast the chosen exiled sorcery and verify it resolves!
    engine
        .apply(me(), PlayerAction::CastSpell { card: spark })
        .expect("cast chosen exiled Spark");
    resolve_top_of_stack(&mut engine);
    assert_eq!(engine.state().players[0].life, 23);
    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Graveyard);
}

// -----------------------------------------------------------------------------
// 2. Queen Kayla-style Hand ChooseCardEx + reveal vs hidden-info + clamping
// -----------------------------------------------------------------------------

#[test]
fn hand_choose_card_ex_reveals_only_when_requested_and_clamps_bounds() {
    static SMALL_PERM: Filter = Filter::And(&[
        Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]),
        Filter::CmcAtMost(2),
    ]);
    static KAYLA_EFFECTS: &[Effect] = &[Effect::ChooseCardEx {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::You),
        filter: &SMALL_PERM,
        min: 3,
        max: Amount::Fixed(3),
        reveal_chosen: true,
        movement: ChooseMove::Battlefield { tapped: false },
        not_chosen_movement: ChooseMove::Graveyard,
        then: &[Effect::gain_life(2)],
        for_unchosen: &[Effect::LoseLife {
            target: PlayerRel::You,
            amount: Amount::Fixed(1),
        }],
    }];
    static PRIVATE_EFFECTS: &[Effect] = &[Effect::ChooseCardEx {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::You),
        filter: &Filter::Any,
        min: 1,
        max: Amount::Fixed(1),
        reveal_chosen: false,
        movement: ChooseMove::None,
        not_chosen_movement: ChooseMove::None,
        then: &[Effect::gain_life(1)],
        for_unchosen: &[],
    }];
    static KAYLA_ABILS: &[AbilityDef] = &[free(KAYLA_EFFECTS, None)];
    static PRIVATE_ABILS: &[AbilityDef] = &[free(PRIVATE_EFFECTS, None)];

    let cards = vec![
        card(
            KAYLA_PROBE,
            face("Kayla Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            KAYLA_ABILS,
        ),
        card(
            PRIVATE_PROBE,
            face("Private Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PRIVATE_ABILS,
        ),
        card(
            RELIC_ONE,
            face("Relic One", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            KNIGHT_TWO,
            creature_face("Knight Two", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            GIANT_FIVE,
            creature_face("Giant Five", "{5}", 5, 5),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1302,
        cards,
        [
            Seat::with(&[KAYLA_PROBE, PRIVATE_PROBE]).holding(&[RELIC_ONE, KNIGHT_TWO, GIANT_FIVE]),
            Seat::default(),
        ],
    );
    let relic = objects(&engine, ZoneLocation::Hand(me()), RELIC_ONE)[0];
    let knight = objects(&engine, ZoneLocation::Hand(me()), KNIGHT_TWO)[0];
    let giant = objects(&engine, ZoneLocation::Hand(me()), GIANT_FIVE)[0];
    let private_probe = objects(&engine, ZoneLocation::Battlefield, PRIVATE_PROBE)[0];
    let kayla_probe = objects(&engine, ZoneLocation::Battlefield, KAYLA_PROBE)[0];

    // Part A: `reveal_chosen: false` + `ChooseMove::None` must record NO `Revealed` event.
    let seq_before_private = engine.state().journal.entries().len();
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: private_probe,
                ability_index: 0,
            },
        )
        .expect("activate Private Probe");
    resolve_top_of_stack(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .expect("privately choose Giant Five");
    let any_revealed_private = engine.state().journal.entries()[seq_before_private..]
        .iter()
        .any(|e| matches!(e.event, GameEvent::Revealed { .. }));
    assert!(
        !any_revealed_private,
        "reveal_chosen: false with ChooseMove::None must not leak a Revealed event"
    );
    assert_eq!(engine.state().players[0].life, 21);

    // Part B: 2 eligible hand cards (`relic`, `knight`), so CR 609.3 clamps `min: 3, max: 3`
    // down to `min: 2, max: 2` while excluding `giant` (MV 5).
    let seq_before_kayla = engine.state().journal.entries().len();
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: kayla_probe,
                ability_index: 0,
            },
        )
        .expect("activate Kayla Probe");
    resolve_top_of_stack(&mut engine);

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards from hand");
    };
    assert_eq!(
        options,
        vec![relic, knight],
        "Giant Five (MV 5) must be excluded by filter"
    );
    assert_eq!(min, 2, "CR 609.3 clamps min=3 down to available=2");
    assert_eq!(max, 2, "CR 609.3 clamps max=3 down to available=2");

    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic, knight],
            },
        )
        .expect("choose both eligible hand cards");

    // Both chosen cards were revealed (and Giant Five was never revealed!).
    let revealed_cards: Vec<ObjectId> = engine.state().journal.entries()[seq_before_kayla..]
        .iter()
        .filter_map(|e| match &e.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(revealed_cards, vec![relic, knight]);
    assert!(!revealed_cards.contains(&giant));
    assert_eq!(
        engine.state().object(relic).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(knight).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(giant).unwrap().zone, Zone::Hand);
    // `then` ran once (+2 life -> 23); `for_unchosen` did NOT run because 0 eligible cards went unchosen!
    assert_eq!(engine.state().players[0].life, 23);
}

// -----------------------------------------------------------------------------
// 3. Killing Wave per-player chooser scope (`ControlledBy(EventPlayer)` & `ControlledByChooser`)
// -----------------------------------------------------------------------------

#[test]
fn killing_wave_per_player_chooser_scope_isolates_each_seats_creatures() {
    static EVENT_PLAYER_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledBy(PlayerRel::EventPlayer),
    ]);
    static CHOOSER_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByChooser]);

    static WAVE_EFFECTS: &[Effect] = &[Effect::RepeatEach {
        domain: RepeatDomain::EachPlayer,
        effects: &[Effect::ChooseCardEx {
            chooser: PlayerRel::EventPlayer,
            zone: ChooseZone::Battlefield,
            filter: &EVENT_PLAYER_CREATURE,
            min: 0,
            max: Amount::Fixed(10),
            reveal_chosen: false,
            movement: ChooseMove::None,
            not_chosen_movement: ChooseMove::None,
            then: &[Effect::LoseLife {
                target: PlayerRel::EventPlayer,
                amount: Amount::Fixed(2),
            }],
            for_unchosen: &[Effect::RepeatEachObject {
                zone: ZoneSel::Battlefield,
                filter: &Filter::Any,
                only_found: true,
                then: &[Effect::SacrificeEvent],
            }],
        }],
    }];
    static OPP_CHOOSER_EFFECTS: &[Effect] = &[Effect::ChooseCardEx {
        chooser: PlayerRel::Opponent,
        zone: ChooseZone::Battlefield,
        filter: &CHOOSER_CREATURE,
        min: 1,
        max: Amount::Fixed(1),
        reveal_chosen: false,
        movement: ChooseMove::Exile,
        not_chosen_movement: ChooseMove::None,
        then: &[],
        for_unchosen: &[],
    }];
    static WAVE_ABILS: &[AbilityDef] = &[free(WAVE_EFFECTS, None)];
    static OPP_CHOOSER_ABILS: &[AbilityDef] = &[free(OPP_CHOOSER_EFFECTS, None)];

    let cards = vec![
        card(
            KILLING_WAVE_PROBE,
            face("Killing Wave Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            WAVE_ABILS,
        ),
        card(
            CHOOSER_SCOPE_PROBE,
            face("Chooser Scope Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            OPP_CHOOSER_ABILS,
        ),
        card(
            TOKEN_A,
            creature_face("Alpha Bear", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_B,
            creature_face("Beta Wolf", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_C,
            creature_face("Gamma Lion", "{3}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let seat2 = PlayerId::new(2);
    let mut engine = bench(
        1303,
        cards,
        [
            Seat::with(&[KILLING_WAVE_PROBE, CHOOSER_SCOPE_PROBE, TOKEN_A]),
            Seat::with(&[TOKEN_B, TOKEN_C]),
            Seat::with(&[TOKEN_A]),
        ],
    );
    let p0_bear = objects(&engine, ZoneLocation::Battlefield, TOKEN_A)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let p2_bear = objects(&engine, ZoneLocation::Battlefield, TOKEN_A)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == seat2)
        .unwrap();
    let p1_wolf = objects(&engine, ZoneLocation::Battlefield, TOKEN_B)[0];
    let p1_lion = objects(&engine, ZoneLocation::Battlefield, TOKEN_C)[0];

    // First verify `Filter::ControlledByChooser` in a 3-player game: `them()` (seat 1)
    // is the first opponent in APNAP order and must ONLY be offered seat 1's creatures
    // (`p1_wolf`, `p1_lion`), NOT seat 2's `p2_bear` (even though seat 2 is also an opponent!).
    let chooser_probe = objects(&engine, ZoneLocation::Battlefield, CHOOSER_SCOPE_PROBE)[0];
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: chooser_probe,
                ability_index: 0,
            },
        )
        .expect("activate Chooser Scope Probe");
    resolve_top_of_stack(&mut engine);

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards for seat 1");
    };
    assert_eq!(player, them());
    assert_eq!(options, vec![p1_wolf, p1_lion]);
    assert!(!options.contains(&p2_bear));
    assert!(!options.contains(&p0_bear));
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![p1_wolf],
            },
        )
        .expect("seat 1 exiles Wolf");
    assert_eq!(engine.state().object(p1_wolf).unwrap().zone, Zone::Exile);

    // Now fire Killing Wave Probe across all 3 seats in APNAP order (seat 0 -> seat 1 -> seat 2):
    let wave_probe = objects(&engine, ZoneLocation::Battlefield, KILLING_WAVE_PROBE)[0];
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: wave_probe,
                ability_index: 0,
            },
        )
        .expect("activate Killing Wave Probe");
    resolve_top_of_stack(&mut engine);

    // Seat 0 offered ONLY `p0_bear` -> chooses to keep it (loses 2 life).
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected seat 0 Killing Wave choice");
    };
    assert_eq!(player, me());
    assert_eq!(options, vec![p0_bear]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![p0_bear],
            },
        )
        .expect("seat 0 keeps p0_bear");

    // Seat 1 offered ONLY `p1_lion` -> chooses `[]` (declines, sacrificing `p1_lion`, losing 0 life).
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected seat 1 Killing Wave choice");
    };
    assert_eq!(player, them());
    assert_eq!(options, vec![p1_lion]);
    engine
        .apply(them(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("seat 1 declines and sacrifices p1_lion");

    // Seat 2 offered ONLY `p2_bear` -> chooses `[]` (sacrifices `p2_bear`).
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected seat 2 Killing Wave choice");
    };
    assert_eq!(player, seat2);
    assert_eq!(options, vec![p2_bear]);
    engine
        .apply(seat2, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("seat 2 declines and sacrifices p2_bear");

    assert_eq!(
        engine.state().object(p0_bear).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(p1_lion).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(p2_bear).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[2].life, 20);
}

// -----------------------------------------------------------------------------
// 4. Liliana of the Veil `-6`: battlefield `SeparateIntoTwoPiles` sacrifice
// -----------------------------------------------------------------------------

#[test]
fn liliana_battlefield_two_piles_sacrifices_only_chosen_pile_and_handles_empty_pile() {
    static CHOSEN_PLAYER_PERMS: Filter = Filter::ControlledBy(PlayerRel::Chosen);
    static LILIANA_EFFECTS: &[Effect] = &[Effect::SeparateIntoTwoPiles {
        separator: PlayerRel::You,
        pile_chooser: PlayerRel::Chosen,
        zone: ZoneSel::Battlefield,
        filter: &CHOSEN_PLAYER_PERMS,
        only_found: false,
        chosen_pile: &[Effect::RepeatEachObject {
            zone: ZoneSel::Battlefield,
            filter: &Filter::Any,
            only_found: true,
            then: &[Effect::SacrificeEvent],
        }],
        unchosen_pile: &[],
    }];
    static LILIANA_ABILS: &[AbilityDef] = &[free(
        LILIANA_EFFECTS,
        Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )];

    let cards = vec![
        card(
            LILIANA_ULT_PROBE,
            face("Liliana Ult Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            LILIANA_ABILS,
        ),
        card(
            TOKEN_A,
            creature_face("Opp Bear", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_B,
            face("Opp Relic", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_C,
            creature_face("Opp Giant", "{5}", 5, 5),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1304,
        cards,
        [
            Seat::with(&[LILIANA_ULT_PROBE]),
            Seat::with(&[TOKEN_A, TOKEN_B, TOKEN_C]),
        ],
    );
    let probe = objects(&engine, ZoneLocation::Battlefield, LILIANA_ULT_PROBE)[0];
    let opp_bear = objects(&engine, ZoneLocation::Battlefield, TOKEN_A)[0];
    let opp_relic = objects(&engine, ZoneLocation::Battlefield, TOKEN_B)[0];
    let opp_giant = objects(&engine, ZoneLocation::Battlefield, TOKEN_C)[0];

    // Activation 1: separate into `[opp_giant]` vs `[opp_bear, opp_relic]`; opponent chooses
    // pile 1 (`[opp_bear, opp_relic]`) to sacrifice so `opp_giant` survives!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate Liliana Ult Probe");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent");
    resolve_top_of_stack(&mut engine);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected FirstPile separation prompt");
    };
    assert_eq!(player, me());
    assert_eq!(prompt, ChoicePrompt::FirstPile);
    assert_eq!(min, 0);
    assert_eq!(max, 3);
    assert_eq!(options, vec![opp_bear, opp_relic, opp_giant]);
    assert!(
        !options.contains(&probe),
        "controller's own permanents must not be in target player's piles"
    );

    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![opp_giant],
            },
        )
        .expect("put Opp Giant in pile 0");

    let Pending::ChoosePile {
        player,
        piles,
        label,
    } = engine.pending().clone()
    else {
        panic!("expected ChoosePile for target player");
    };
    assert_eq!(player, them());
    assert_eq!(label, None);
    assert_eq!(piles, vec![vec![opp_giant], vec![opp_bear, opp_relic]]);

    engine
        .apply(them(), PlayerAction::ChooseMode(1))
        .expect("opponent chooses pile 1 to sacrifice");

    assert_eq!(
        engine.state().object(opp_giant).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(opp_bear).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(opp_relic).unwrap().zone,
        Zone::Graveyard
    );

    // Activation 2 (empty-pile edge case per CR 700.3): separate `[opp_giant]` into
    // `[]` (pile 0) and `[opp_giant]` (pile 1); opponent chooses pile 0 (`[]`) and sacrifices nothing!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate Liliana Ult Probe second time");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent again");
    resolve_top_of_stack(&mut engine);

    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("make pile 0 empty");
    let Pending::ChoosePile { piles, .. } = engine.pending().clone() else {
        panic!("expected ChoosePile with empty pile 0");
    };
    assert_eq!(piles, vec![Vec::<ObjectId>::new(), vec![opp_giant]]);
    engine
        .apply(them(), PlayerAction::ChooseMode(0))
        .expect("opponent chooses empty pile 0");
    assert_eq!(
        engine.state().object(opp_giant).unwrap().zone,
        Zone::Battlefield,
        "choosing the empty pile sacrifices zero permanents"
    );
}

// -----------------------------------------------------------------------------
// 5. Jace, Architect of Thought `-2` & Hostile Negotiations `Found` TwoPiles
// -----------------------------------------------------------------------------

#[test]
fn jace_and_hostile_negotiations_found_two_piles_and_empty_pile_edges() {
    // Jace `-2`: Reveal top 3 cards, opponent separates into 2 piles, put chosen pile
    // into hand and unchosen pile on bottom of library in any order.
    static JACE_EFFECTS: &[Effect] = &[Effect::RevealUntilThen {
        library: PlayerRel::You,
        filter: &Filter::Any,
        count: Amount::Fixed(3),
        max_revealed: Some(Amount::Fixed(3)),
        exile_while_revealing: false,
        found: RevealUntilFound::StayInLibrary,
        rest: RevealUntilRest::StayInLibrary,
        then: &[Effect::SeparateIntoTwoPiles {
            separator: PlayerRel::Opponent,
            pile_chooser: PlayerRel::You,
            zone: ZoneSel::LibraryYou,
            filter: &Filter::Any,
            only_found: true,
            chosen_pile: &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Found,
                filter: &Filter::Any,
                min: 0,
                max: Amount::Fixed(0),
                reveal_chosen: false,
                movement: ChooseMove::None,
                not_chosen_movement: ChooseMove::Hand,
                then: &[],
                for_unchosen: &[],
            }],
            unchosen_pile: &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Found,
                filter: &Filter::Any,
                min: 0,
                max: Amount::Fixed(0),
                reveal_chosen: false,
                movement: ChooseMove::None,
                not_chosen_movement: ChooseMove::BottomOfLibraryChosenOrder,
                then: &[],
                for_unchosen: &[],
            }],
        }],
    }];

    // Hostile Negotiations-style exiled `Found` TwoPiles: separate exiled `Found` cards into
    // two piles, turn chosen pile face up, then opponent chooses which pile goes to hand vs graveyard!
    static NEGOTIATIONS_EFFECTS: &[Effect] = &[Effect::SeparateIntoTwoPiles {
        separator: PlayerRel::You,
        pile_chooser: PlayerRel::Opponent,
        zone: ZoneSel::ExileAll,
        filter: &Filter::Any,
        only_found: true,
        chosen_pile: &[
            Effect::RepeatEachObject {
                zone: ZoneSel::ExileAll,
                filter: &Filter::Any,
                only_found: true,
                then: &[Effect::TurnFaceUp {
                    target: TargetSpec::EventObject,
                }],
            },
            Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Found,
                filter: &Filter::Any,
                min: 0,
                max: Amount::Fixed(0),
                reveal_chosen: false,
                movement: ChooseMove::None,
                not_chosen_movement: ChooseMove::Hand,
                then: &[],
                for_unchosen: &[],
            },
        ],
        unchosen_pile: &[Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone: ChooseZone::Found,
            filter: &Filter::Any,
            min: 0,
            max: Amount::Fixed(0),
            reveal_chosen: false,
            movement: ChooseMove::None,
            not_chosen_movement: ChooseMove::Graveyard,
            then: &[],
            for_unchosen: &[],
        }],
    }];

    static JACE_ABILS: &[AbilityDef] = &[free(JACE_EFFECTS, None)];
    static NEGOTIATIONS_ABILS: &[AbilityDef] = &[free(
        &[Effect::RevealUntilThen {
            library: PlayerRel::You,
            filter: &Filter::Any,
            count: Amount::Fixed(3),
            max_revealed: Some(Amount::Fixed(3)),
            exile_while_revealing: true,
            found: RevealUntilFound::Exile,
            rest: RevealUntilRest::Exile,
            then: NEGOTIATIONS_EFFECTS,
        }],
        None,
    )];

    let cards = vec![
        card(
            JACE_PILES_PROBE,
            face("Jace Piles Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            JACE_ABILS,
        ),
        card(
            NEGOTIATIONS_PROBE,
            face("Negotiations Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            NEGOTIATIONS_ABILS,
        ),
        card(
            TOKEN_A,
            creature_face("Card Alpha", "{1}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_B,
            creature_face("Card Beta", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_C,
            creature_face("Card Gamma", "{3}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        1305,
        cards,
        [
            Seat::with(&[JACE_PILES_PROBE, NEGOTIATIONS_PROBE])
                .holding(&[TOKEN_A, TOKEN_B, TOKEN_C]),
            Seat::default(),
        ],
    );
    let c_alpha = put_hand_card_on_library_top(&mut engine, me(), TOKEN_A);
    let c_beta = put_hand_card_on_library_top(&mut engine, me(), TOKEN_B);
    let c_gamma = put_hand_card_on_library_top(&mut engine, me(), TOKEN_C);
    refresh_offer(&mut engine);

    let jace_probe = objects(&engine, ZoneLocation::Battlefield, JACE_PILES_PROBE)[0];
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: jace_probe,
                ability_index: 0,
            },
        )
        .expect("activate Jace Piles Probe");
    resolve_top_of_stack(&mut engine);

    // Opponent separates `[c_gamma, c_beta, c_alpha]` into `[c_gamma]` and `[c_beta, c_alpha]`.
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected opponent FirstPile prompt");
    };
    assert_eq!(player, them());
    assert_eq!(prompt, ChoicePrompt::FirstPile);
    assert_eq!(options, vec![c_gamma, c_beta, c_alpha]);
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![c_gamma],
            },
        )
        .expect("opponent separates c_gamma into pile 0");

    // Controller chooses pile 0 (`[c_gamma]`), so `c_gamma` goes to hand and `[c_beta, c_alpha]`
    // asks `Pending::Arrange` for bottom-of-library order!
    let Pending::ChoosePile { player, piles, .. } = engine.pending().clone() else {
        panic!("expected controller ChoosePile");
    };
    assert_eq!(player, me());
    assert_eq!(piles, vec![vec![c_gamma], vec![c_beta, c_alpha]]);
    engine
        .apply(me(), PlayerAction::ChooseMode(0))
        .expect("controller chooses pile 0");

    assert_eq!(engine.state().object(c_gamma).unwrap().zone, Zone::Hand);
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        panic!("expected Arrange for 2-card unchosen pile on bottom of library");
    };
    assert_eq!(player, me());
    assert_eq!(cards, vec![c_beta, c_alpha]);
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![vec![c_alpha, c_beta]],
            },
        )
        .expect("order c_alpha then c_beta (c_beta very bottom)");

    let lib = engine.state().zones.list(ZoneLocation::Library(me()));
    assert_eq!(lib[0], c_beta, "last arranged card sits on very bottom");
    assert_eq!(
        lib[1], c_alpha,
        "first arranged card sits directly above it"
    );

    // Part B: Empty-pile + face-down Exile `TurnFaceUp` + opponent-chooses-pile test!
    // Put `c_gamma` back on top, plus two Forests already on top, and conceal one in exile
    // during separation to verify `TurnFaceUp` on face-down exile + empty-pile selection!
    let _ = engine.state.move_object(
        c_gamma,
        ZoneLocation::Library(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    let neg_probe = objects(&engine, ZoneLocation::Battlefield, NEGOTIATIONS_PROBE)[0];
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: neg_probe,
                ability_index: 0,
            },
        )
        .expect("activate Negotiations Probe");
    resolve_top_of_stack(&mut engine);

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected controller FirstPile over exiled Found cards");
    };
    assert_eq!(player, me());
    assert_eq!(options.len(), 3);
    assert_eq!(options[0], c_gamma);
    // Conceal `c_gamma` face down in exile (as Hostile Negotiations does) before choosing piles:
    engine.state.conceal_exiled_card(c_gamma);
    assert!(
        engine
            .state()
            .object(c_gamma)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    // Put ALL 3 exiled cards into pile 0 (`options.clone()` vs `[]`), then have opponent
    // choose the EMPTY pile 1 (`[]`): 0 cards go to hand, and all 3 unchosen cards go to graveyard!
    let all_three = options.clone();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: all_three.clone(),
            },
        )
        .expect("put all three into pile 0, leaving pile 1 empty");

    let Pending::ChoosePile { player, piles, .. } = engine.pending().clone() else {
        panic!("expected opponent ChoosePile");
    };
    assert_eq!(player, them());
    assert_eq!(piles, vec![all_three.clone(), vec![]]);
    engine
        .apply(them(), PlayerAction::ChooseMode(1))
        .expect("opponent chooses empty pile 1");

    for id in all_three {
        assert_eq!(
            engine.state().object(id).unwrap().zone,
            Zone::Graveyard,
            "when empty pile is chosen, all cards in the unchosen pile still move to graveyard"
        );
        assert!(
            !engine
                .state()
                .object(id)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN),
            "cards moved from face-down exile to graveyard enter face up"
        );
    }
}

// -----------------------------------------------------------------------------
// 6. LinkedExile & face-down Exile TurnFaceUp + 3-player TwoPiles opponent picker
// -----------------------------------------------------------------------------

#[test]
fn linked_exile_choose_zone_facedown_turn_face_up_and_multiplayer_two_piles() {
    static EXILE_LINKED_ABILS: &[AbilityDef] = &[
        free(
            &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Hand(PlayerRel::You),
                filter: &Filter::Any,
                min: 2,
                max: Amount::Fixed(2),
                reveal_chosen: false,
                movement: ChooseMove::ExileLinked,
                not_chosen_movement: ChooseMove::None,
                then: &[],
                for_unchosen: &[],
            }],
            None,
        ),
        free(
            &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::LinkedExile,
                filter: &Filter::Any,
                min: 1,
                max: Amount::Fixed(1),
                reveal_chosen: false,
                movement: ChooseMove::Hand,
                not_chosen_movement: ChooseMove::Graveyard,
                then: &[],
                for_unchosen: &[],
            }],
            None,
        ),
        free(
            &[Effect::SeparateIntoTwoPiles {
                separator: PlayerRel::Opponent,
                pile_chooser: PlayerRel::Opponent,
                zone: ZoneSel::ExileAll,
                filter: &Filter::Any,
                only_found: false,
                chosen_pile: &[Effect::RepeatEachObject {
                    zone: ZoneSel::ExileAll,
                    filter: &Filter::Any,
                    only_found: true,
                    then: &[Effect::TurnFaceUp {
                        target: TargetSpec::EventObject,
                    }],
                }],
                unchosen_pile: &[],
            }],
            None,
        ),
    ];

    let cards = vec![
        card(
            MENTOR_PROBE,
            face("Vault Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            EXILE_LINKED_ABILS,
        ),
        card(
            TOKEN_A,
            creature_face("Alpha Card", "{1}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TOKEN_B,
            creature_face("Beta Card", "{2}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let seat2 = PlayerId::new(2);
    let mut engine = bench(
        1306,
        cards,
        [
            Seat::with(&[MENTOR_PROBE]).holding(&[TOKEN_A, TOKEN_B]),
            Seat::default(),
            Seat::default(),
        ],
    );
    let probe = objects(&engine, ZoneLocation::Battlefield, MENTOR_PROBE)[0];
    let alpha = objects(&engine, ZoneLocation::Hand(me()), TOKEN_A)[0];
    let beta = objects(&engine, ZoneLocation::Hand(me()), TOKEN_B)[0];

    // Ability 0: exile `alpha` and `beta` linked to `probe` via `ChooseMove::ExileLinked`.
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 0,
            },
        )
        .expect("activate exile-linked");
    resolve_top_of_stack(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![alpha, beta],
            },
        )
        .expect("exile both linked");
    assert_eq!(engine.state().object(alpha).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(beta).unwrap().zone, Zone::Exile);

    // Conceal both face down in exile, then run Ability 2 in 3-player game:
    // controller picks seat 2 to separate, then picks seat 1 to choose a pile,
    // turning the chosen pile face up IN EXILE while the unchosen pile stays face down!
    engine.state.conceal_exiled_card(alpha);
    engine.state.conceal_exiled_card(beta);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 2,
            },
        )
        .expect("activate 3p TwoPiles");
    resolve_top_of_stack(&mut engine);

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("expected ChoosePlayer for separator in 3p game");
    };
    assert_eq!(player, me());
    assert_eq!(options, vec![them(), seat2]);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(seat2))
        .expect("pick seat 2 as separator");

    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!("expected seat 2 FirstPile");
    };
    assert_eq!(player, seat2);
    engine
        .apply(
            seat2,
            PlayerAction::ChooseObjects {
                objects: vec![alpha],
            },
        )
        .expect("seat 2 separates alpha vs beta");

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("expected ChoosePlayer for pile_chooser in 3p game");
    };
    assert_eq!(player, me());
    assert_eq!(options, vec![them(), seat2]);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .expect("pick seat 1 as pile chooser");

    let Pending::ChoosePile { player, .. } = engine.pending().clone() else {
        panic!("expected seat 1 ChoosePile");
    };
    assert_eq!(player, them());
    engine
        .apply(them(), PlayerAction::ChooseMode(0))
        .expect("seat 1 chooses pile 0 (alpha) to turn face up");

    assert!(
        !engine
            .state()
            .object(alpha)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "chosen pile card in exile was turned face up in place"
    );
    assert!(
        engine
            .state()
            .object(beta)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "unchosen pile card in exile stays face down"
    );
    // Turn beta back face up so face-up LinkedExile sees both `alpha` and `beta`:
    engine.state.reveal_exiled_card(beta);

    // Ability 1: `ChooseZone::LinkedExile` offers `[alpha, beta]`; choosing `alpha` moves
    // `alpha` to Hand and `beta` (`not_chosen_movement: Graveyard`) to Graveyard!
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: probe,
                ability_index: 1,
            },
        )
        .expect("activate LinkedExile choose");
    resolve_top_of_stack(&mut engine);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected LinkedExile ChooseCards");
    };
    assert_eq!(options, vec![alpha, beta]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![alpha],
            },
        )
        .expect("choose alpha from LinkedExile");
    assert_eq!(engine.state().object(alpha).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().object(beta).unwrap().zone, Zone::Graveyard);
}
