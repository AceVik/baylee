//! Mana production, now that one effect covers all of it.
//!
//! Seven `Effect` variants used to each fix all three of the independent
//! questions a mana line asks — which colors, how much, and what it may be
//! spent on — and none of the cards below had an engine test. They do now,
//! one per answer the unified effect has to get right.

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, pass_until, reach_main_phase,
};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}
fn mystic_gate() -> CardIndex {
    card_index("e9f5feb2-2c1a-46ce-885a-4f378d7d10af")
}
fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}
fn command_tower() -> CardIndex {
    card_index("0895c9b7-ae7d-4bb3-af17-3b75deb50a25")
}
fn cavern_of_souls() -> CardIndex {
    card_index("89ca686a-7c72-4d8f-9290-e89635624a83")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
/// The battlefield permanent a seat's copy of `card` is.
#[track_caller]
fn land_object(engine: &Engine<RegistryLookup>, card: CardIndex) -> baylee_core::ids::ObjectId {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the permanent is on the battlefield")
}
fn kazandu_blademaster() -> CardIndex {
    card_index("133f5d30-d883-493e-93a1-cf9583db460b")
}
fn charming_prince() -> CardIndex {
    card_index("c48d844c-3976-4fa5-8e0d-3f0e535e7619")
}
fn reflecting_pool() -> CardIndex {
    card_index("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
}
fn badlands() -> CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}

/// Activates printed ability `index` of `card`.
#[track_caller]
fn chromatic_sphere() -> CardIndex {
    card_index("2e03e44a-9fff-4490-859f-b42e89e8563a")
}
fn talisman_of_dominance() -> CardIndex {
    card_index("4c0a0448-b9d6-43a0-8549-64066dac63f0")
}
fn talisman_of_progress() -> CardIndex {
    card_index("00e35322-1a9a-41e3-9ce1-359c8eaa3bc7")
}
fn grove_of_the_burnwillows() -> CardIndex {
    card_index("d33c3fbb-8306-4c2d-b0dd-88f12639da94")
}
fn fogwells_gym() -> CardIndex {
    card_index("850bb6f7-48d3-4d65-9220-b0bec5ee6b64")
}

fn activate(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex, index: u32) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, ai)| {
            *ai == index
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the ability is offered");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability activates");
}

/// "Add two mana in any combination of {W} and/or {U}." Two mana is two
/// picks — the old non-combination path added the whole amount per pick and
/// then asked again, which paid X² mana for a line that promises X.
///
/// The subject used to be Harabaz Druid, which was written with
/// `mana_combination` and prints "any **one** color"; the test passed on a
/// card the rule does not apply to, and the pair below is what separates the
/// two sentences now.
#[test]
fn any_combination_adds_one_mana_per_pick() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[mystic_gate(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The gate's combination line costs `{W/U}, {T}` — one mana of *either*
    // colour, which is the point of a filter land and is why the fixture is a
    // Plains rather than the Forest that stood here while the card was
    // written with a generic `{1}`. An ability whose mana is not floating is
    // not offered at all, so the basic goes first.
    activate(&mut engine, p0, plains(), 0);
    activate(&mut engine, p0, mystic_gate(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "white or blue");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("first mana");
    assert!(
        matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the second mana is a second pick, not a repeat of the first"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("second mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 2, "the {{G}} was spent on the {{1}}");
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
}

/// "Add X mana of any **one** color, where X is the number of Allies you
/// control." Two Allies is two mana and **one** pick, both of that colour.
///
/// Reported from a game: the Druid was asking once per mana and making
/// {W}{U} where it prints one colour. It was written with
/// [`Effect::mana_combination`](baylee_cards_dsl::Effect::mana_combination),
/// whose whole job is the opposite sentence — and the shape it wanted,
/// a pick for a counted amount, was not sayable until
/// `Effect::mana_choice_dynamic` existed.
#[test]
fn x_mana_of_any_one_color_is_one_pick_for_all_of_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[harabaz_druid(), halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, harabaz_druid(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any of the five colours");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("the colour");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one colour for the whole of X, so there is no second question: {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 2, "two Allies, two mana");
    assert_eq!(pool.available(ManaColor::Green), 2, "both of one colour");
}

/// "Add one mana of any color in your commander's color identity." There is
/// no commander in a duel, so there is no color to choose and the ability
/// still has to resolve — colorless is what is left of it.
#[test]
fn command_tower_without_a_commander_makes_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[command_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, command_tower(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "one option is no choice: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
}

/// Reported from a game: a Cavern of Souls naming Ally, two of them
/// untapped, and Kazandu Blademaster sat in hand refusing to be cast.
///
/// The mana was made and the mana matched. What could not see it was
/// `casting::can_cast` — restricted mana is not in the pool's plain counters
/// and `mana_pay::can_pay` reads nothing else, so the spell was never
/// *offered*, while the payment on the far side of the cast wizard would
/// have paid for it without complaint. Both halves ask the same
/// question now, and the counter-half of this test is the one that says the
/// answer is still a restriction: Charming Prince is no Ally and the same
/// two mana buy it nothing.
#[test]
fn a_cavern_naming_ally_pays_for_an_ally() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls(), plains()])
        .hand(0, &[kazandu_blademaster(), charming_prince()])
        .start();
    // The Cavern's "choose a creature type" lands in among the mulligans.
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::ChooseSubtype { player, options } => {
                let ally = options
                    .iter()
                    .copied()
                    .find(|s| *s == baylee_core::generated::subtypes::creature::ALLY)
                    .expect("Ally is a creature type a Cavern may name");
                engine
                    .apply(player, PlayerAction::ChooseSubtype(ally))
                    .expect("a creature type is chosen");
            }
            other => panic!("expected a mulligan or the type choice, got {other:?}"),
        }
    }
    reach_main_phase(&mut engine, p0);

    // One white off the Cavern's restricted ability, one off the Plains, so
    // the two halves of the cost are paid from the two halves of the pool.
    activate(&mut engine, p0, cavern_of_souls(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("colour chosen");
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: land_object(&engine, plains()),
            },
        )
        .expect("a Plains taps for white");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "the Plains' white");
    let restricted: u16 = pool.restricted().iter().map(|m| m.amount).sum();
    assert_eq!(restricted, 1, "the Cavern's white, still restricted");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let named = |id: baylee_core::ids::ObjectId| {
        engine
            .state()
            .object(id)
            .and_then(|o| o.card)
            .map(|c| c.index)
    };
    let castable: Vec<_> = legal.castable.iter().copied().filter_map(named).collect();
    assert!(
        castable.contains(&kazandu_blademaster()),
        "an Ally is what this mana is for, and {{W}}{{W}} of it is floating"
    );
    assert!(
        !castable.contains(&charming_prince()),
        "a Human Noble is not an Ally — the restriction still holds"
    );

    // And it is an offer that survives being taken.
    let spell = legal
        .castable
        .iter()
        .copied()
        .find(|id| named(*id) == Some(kazandu_blademaster()))
        .expect("the Blademaster is offered");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the Blademaster is cast with the Cavern's mana");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both restricted mana were spent on it"
    );
}

/// "Spend this mana only to cast a creature spell of the chosen type, and
/// that spell can't be countered." The rider rides on the mana, so it has
/// to survive the color choice: the pool keeps it as restricted mana, not
/// as an ordinary red.
#[test]
fn cavern_of_souls_mana_stays_restricted_after_the_colour_choice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls()])
        .start();
    // "As this land enters, choose a creature type" — the preset puts it on
    // the battlefield, so that choice lands in the middle of the mulligans.
    for _ in 0..3 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::ChooseSubtype { player, options } => {
                engine
                    .apply(player, PlayerAction::ChooseSubtype(options[0]))
                    .expect("a creature type is chosen");
            }
            other => panic!("expected a mulligan or the type choice, got {other:?}"),
        }
    }
    reach_main_phase(&mut engine, p0);

    // Ability 0 is the plain {C}; ability 1 is the restricted any-colour.
    activate(&mut engine, p0, cavern_of_souls(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("colour chosen");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "restricted mana is not free red"
    );
    let restricted = pool.restricted();
    assert_eq!(restricted.len(), 1);
    assert_eq!(restricted[0].color, ManaColor::Red);
    assert_eq!(restricted[0].amount, 1);
    assert!(
        engine
            .state()
            .restriction_info
            .contains_key(&restricted[0].restriction.0),
        "the spend restriction is registered, or nothing can check it"
    );
}

/// "Add one mana of any type that a land you control could produce." A
/// Reflecting Pool reads that off the *other* lands — it produces nothing
/// by itself, so it must not read its own promise back. Next to a Badlands
/// it makes black or red, and nothing else.
#[test]
fn reflecting_pool_offers_what_the_other_lands_produce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(14, forest())
        .battlefield(0, &[reflecting_pool(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, reflecting_pool(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the Badlands' two colours, not the rainbow the Pool would promise itself"
    );
}

/// With no other land, the Pool has nothing to reflect and produces no mana
/// at all — the ability still resolves (CR 106.5), it just adds nothing.
#[test]
fn a_lone_reflecting_pool_produces_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(15, forest())
        .battlefield(0, &[reflecting_pool()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, reflecting_pool(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "nothing to choose from: {:?}",
        engine.pending()
    );
    assert!(
        engine.state().players[0].mana_pool.is_empty(),
        "a Pool reflecting only itself is not a rainbow land"
    );
}

/// Taps the first mana source the seat is offered.
fn tap_a_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let source = *legal
        .mana_abilities
        .first()
        .expect("an untapped mana source");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("the land taps");
}

/// CR 500.5: "When a step or phase ends, any unused mana left in a player's
/// mana pool empties." CR 106.4 says the same thing from the other side —
/// "Each player's mana pool empties at the end of each step and phase."
///
/// `ManaPool::empty_at_step_end` was written for exactly this and had one
/// caller in the whole workspace: its own unit test. Nothing in the engine
/// ever emptied a pool, so a Forest tapped in the first main phase was still
/// paying for things in the end step, on the opponent's turn, and three
/// turns later.
#[test]
fn a_pool_does_not_survive_the_step_the_mana_was_made_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(13, forest()).battlefield(0, &[forest()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_a_land(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the premise: one green is floating"
    );
    let made_in = engine.state().turn.step;
    pass_until(&mut engine, |e| e.state().turn.step != made_in);
    assert!(
        engine.state().players[0].mana_pool.is_empty(),
        "the main phase ended and the mana was still there: {:?}",
        engine.state().players[0].mana_pool
    );
}

/// The same rule at the coarsest boundary there is, because this is the one
/// that has already cost a test its premise: `cast_face_tests` had to spend
/// every point it floated before passing the turn, or the next turn's
/// question was answered with the last turn's mana.
#[test]
fn a_pool_does_not_survive_the_turn_the_mana_was_made_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest()).battlefield(0, &[forest()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_a_land(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    let made_in = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > made_in);
    assert!(
        engine.state().players[0].mana_pool.is_empty(),
        "the turn ended and the mana was still there: {:?}",
        engine.state().players[0].mana_pool
    );
}

/// CR 605.1a asks whether an activated ability **could** add mana without
/// targeting — not whether adding mana is the only thing it does.
///
/// Five cards in this pool print a rider beside their mana, and the reader
/// that wrote them asked the wrong question: `.all(…)` over the effects,
/// "every one of these is mana", where the rule asks `.any(…)`. So all five
/// were written `activated!`, and the engine put them on the **stack** — a
/// land tapped for mana became something an opponent could respond to, and
/// the client, which reads the same flag to decide that a mana ability is the
/// one thing it need not confirm (CR 605.1), asked for a second tap.
///
/// This is the rules half of that fix and it fails against the old pool on
/// the very first assertion, because the ability is sitting on the stack
/// instead of having resolved. All five are played here rather than one,
/// because each was written by the same rule and each is a different shape:
/// a flat colour, a choice of two, a choice that pays an *opponent*, and one
/// whose whole cost is paid out of the card itself.
#[test]
fn a_mana_ability_that_does_something_else_too_still_skips_the_stack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(
            0,
            &[
                fogwells_gym(),
                talisman_of_dominance(),
                talisman_of_progress(),
                grove_of_the_burnwillows(),
                chromatic_sphere(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let opponent_before = engine.state().players[1].life;

    // "{T}: Add {R}. This land deals 1 damage to you." — no choice and no
    // cost but the tap, which makes it the cleanest reading of the rule.
    activate(&mut engine, p0, fogwells_gym(), 0);
    assert!(
        super::testkit::stack_is_empty(&engine),
        "a mana ability never uses the stack (CR 605.1), rider or no rider",
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "the rider is not skipped just because the ability skipped the stack",
    );

    // A choice of two, and the damage again.
    activate(&mut engine, p0, talisman_of_dominance(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("blue or black");
    assert!(super::testkit::stack_is_empty(&engine));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert_eq!(engine.state().players[0].life, life_before - 2);

    activate(&mut engine, p0, talisman_of_progress(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white or blue");
    assert!(super::testkit::stack_is_empty(&engine));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert_eq!(engine.state().players[0].life, life_before - 3);

    // The rider that pays somebody else: "Each opponent gains 1 life."
    activate(&mut engine, p0, grove_of_the_burnwillows(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("red or green");
    assert!(super::testkit::stack_is_empty(&engine));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        engine.state().players[1].life,
        opponent_before + 1,
        "the Grove pays the opponent, and it pays them off the stack",
    );

    // "{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a
    // card." The cost is paid out of the four mana the riders just made, so
    // this also shows a mana ability spending a mana ability's own output.
    let pool_before = engine.state().players[0].mana_pool.total();
    assert_eq!(pool_before, 4, "one from each of the four above");
    activate(&mut engine, p0, chromatic_sphere(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any color is five of them");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("any colour");
    assert!(
        super::testkit::stack_is_empty(&engine),
        "sacrificing itself to draw a card is still a mana ability",
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the {{1}} came out of the pool and the {{U}} went back in",
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
}

// ---------------------------------------------------------------------------
// Paying with restricted mana: one solve, charged to the admitted entries.
// ---------------------------------------------------------------------------

fn sea_eagle() -> CardIndex {
    card_index("acb57162-7093-4a3c-9818-d3b61ce757c6")
}
fn mishras_workshop() -> CardIndex {
    card_index("ba284fe6-bb29-455c-8321-9714a0cdc05e")
}
fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}
fn oakhollow_village() -> CardIndex {
    card_index("177b7fe4-8565-4631-b4d9-8b2b4282f3ac")
}
fn goblin_balloon_brigade() -> CardIndex {
    card_index("10bc98b0-3fdc-46d1-8d3b-6d160e9dd62f")
}
fn mycosynth_lattice() -> CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}
fn henge_guardian() -> CardIndex {
    card_index("71744428-65ae-42ac-893d-0802fd41e9b4")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Keeps both hands and answers a Cavern's "choose a creature type" with
/// `named`, whichever order the two arrive in, then walks to seat 0's main
/// phase.
fn settle(engine: &mut Engine<RegistryLookup>, named: baylee_core::ids::SubtypeId) {
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::ChooseSubtype { player, options } => {
                assert!(options.contains(&named), "the Cavern may name it");
                engine
                    .apply(player, PlayerAction::ChooseSubtype(named))
                    .expect("a creature type is chosen");
            }
            other => panic!("expected a mulligan or the type choice, got {other:?}"),
        }
    }
    reach_main_phase(engine, PlayerId::new(0));
}

/// Taps a basic land of seat 0's for its CR 305.6 mana.
fn tap_basic(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    let source = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.card.is_some_and(|c| c.index == card)
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .expect("an untapped copy of the land");
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateManaAbility { source },
        )
        .expect("a basic land taps for its mana");
}

/// The Cavern's restricted ability, answered with `color`.
fn tap_cavern_for(engine: &mut Engine<RegistryLookup>, color: ManaColor) {
    activate(engine, PlayerId::new(0), cavern_of_souls(), 1);
    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseColor(color))
        .expect("colour chosen");
}

/// Casts `card` from seat 0's hand, which has to be offered first.
#[track_caller]
fn cast(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> Result<(), EngineError> {
    let spell = super::testkit::in_hand(engine, PlayerId::new(0), card).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "the spell is offered before it is taken"
    );
    engine.apply(PlayerId::new(0), PlayerAction::CastSpell { card: spell })
}

/// How much restricted mana seat 0 has floating.
fn restricted_total(engine: &Engine<RegistryLookup>) -> u16 {
    engine.state().players[0]
        .mana_pool
        .restricted()
        .iter()
        .map(|m| m.amount)
        .sum()
}

/// A payment that cannot be made leaves the pool exactly as it was, the
/// restricted entries included — partial payments are not allowed
/// (CR 601.2h), and a refused one must not have eaten a Workshop's mana on
/// the way to refusing.
#[test]
fn a_failed_payment_leaves_restricted_mana_where_it_was() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[mishras_workshop()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), mishras_workshop(), 0);
    let ring = super::testkit::in_hand(&engine, PlayerId::new(0), sol_ring()).unwrap();
    let before = engine.state().players[0].mana_pool.clone();
    assert_eq!(restricted_total(&engine), 3, "the Workshop's three");

    let too_much = baylee_core::mana!("{4}");
    let paid = crate::casting::pay_mana_for(
        &mut engine.state,
        PlayerId::new(0),
        crate::casting::SpendFor::Spell(ring),
        &too_much,
    );
    assert!(paid.is_none(), "three mana do not pay four");
    assert_eq!(engine.state().players[0].mana_pool, before);

    // And a payment nothing restricted may serve reads the plain pool
    // alone, which is empty.
    let one = baylee_core::mana!("{1}");
    let paid = crate::casting::pay_mana_for(
        &mut engine.state,
        PlayerId::new(0),
        crate::casting::SpendFor::Other,
        &one,
    );
    assert!(
        paid.is_none(),
        "an artifact spell's mana pays for nothing else"
    );
    assert_eq!(engine.state().players[0].mana_pool, before);
}

/// **A restricted colour pays its own pip, not the generic.** Cavern of
/// Souls naming Bird makes `{U}`, a Forest makes `{G}`, and Sea Eagle costs
/// `{1}{U}`. The payer this replaced took the Cavern's entry whole and
/// subtracted it from the cost generic-first, so the blue paid the `{1}` and
/// the Forest was left to pay `{U}`: the spell was offered and then refused
/// as "cannot pay the total cost".
#[test]
fn a_restricted_colour_pays_its_own_pip_not_the_generic() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls(), forest()])
        .hand(0, &[sea_eagle()])
        .start();
    settle(
        &mut engine,
        baylee_core::generated::subtypes::creature::BIRD,
    );
    tap_cavern_for(&mut engine, ManaColor::Blue);
    tap_basic(&mut engine, forest());

    cast(&mut engine, sea_eagle()).expect("the Cavern's blue pays the blue");
    let spell = super::testkit::on_stack(&engine, sea_eagle()).expect("on the stack");
    assert!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "the Cavern's mana was spent on it, so it can't be countered"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// **Restricted mana a payment did not need stays, and stays restricted.**
/// Mishra's Workshop makes `{C}{C}{C}` for artifact spells and Sol Ring
/// costs `{1}`. The replaced payer took the entry whole, so two colourless
/// vanished into a one-mana spell.
#[test]
fn restricted_mana_left_over_after_a_payment_stays_restricted() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[mishras_workshop()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), mishras_workshop(), 0);
    let before: Vec<_> = engine.state().players[0].mana_pool.restricted().to_vec();
    assert_eq!(before.len(), 1);

    cast(&mut engine, sol_ring()).expect("an artifact spell, paid by the Workshop");
    let after = engine.state().players[0].mana_pool.restricted().to_vec();
    assert_eq!(after.len(), 1, "one entry, not a new one: {after:?}");
    assert_eq!(after[0].amount, 2, "one of the three was spent");
    assert_eq!(after[0].color, ManaColor::Colorless);
    assert_eq!(
        after[0].restriction, before[0].restriction,
        "the two left are still for artifact spells only"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
}

/// **Restricted mana that matches no pip is left alone.** Oakhollow
/// Village's `{G}` is for creature spells, and Goblin Balloon Brigade is
/// one — but it costs `{R}`, which the Mountain pays. The green was admitted,
/// paid nothing, and was taken and lost by the payer this replaced.
#[test]
fn restricted_mana_that_matches_no_pip_is_left_in_the_pool() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[oakhollow_village(), mountain()])
        .hand(0, &[goblin_balloon_brigade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), oakhollow_village(), 1);
    tap_basic(&mut engine, mountain());

    cast(&mut engine, goblin_balloon_brigade()).expect("the Mountain pays the {R}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(restricted_total(&engine), 1, "the green is still there");
    assert_eq!(pool.restricted()[0].color, ManaColor::Green);
    assert_eq!(pool.available(ManaColor::Red), 0, "the red paid");
}

fn boseiju_who_shelters_all() -> CardIndex {
    card_index("36937483-30cb-449a-8028-75017a124922")
}

/// **Mana with a rider alone pays for what its rider does not name** (#232).
/// Boseiju's `{C}` makes an instant or sorcery uncounterable and restricts
/// nothing (CR 106.6), and Sol Ring is neither. Read as a restriction, the
/// `{C}` was admitted for instants and sorceries only, and Sol Ring was
/// offered nothing.
#[test]
fn mana_with_a_rider_alone_pays_for_a_spell_its_rider_does_not_name() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[boseiju_who_shelters_all()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), boseiju_who_shelters_all(), 0);

    cast(&mut engine, sol_ring()).expect("the {C} is ordinary mana");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// **An ordinary spend keeps the rider back.** Boseiju's `{C}` and Command
/// Tower's (no commander, so colourless) are one colour, and Sol Ring needs
/// one of them. The Tower's pays; Boseiju's stays for a spell its rider
/// names, as a player paying by hand would have it.
#[test]
fn a_spell_the_rider_does_not_name_is_paid_with_the_other_mana_first() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[boseiju_who_shelters_all(), command_tower()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), boseiju_who_shelters_all(), 0);
    activate(&mut engine, PlayerId::new(0), command_tower(), 0);

    cast(&mut engine, sol_ring()).expect("either {C} pays");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.ridden().len(), 1, "Boseiju's is the one left");
}

/// **Under Mycosynth Lattice restricted mana pays any pip** (CR 106.6 still
/// decides what it may be spent on, the Lattice decides as what). Oakhollow's
/// green alone casts a `{R}` creature. The replaced payer matched colours
/// literally, so the spell was offered — the probe honours the Lattice —
/// and then refused.
#[test]
fn under_a_lattice_restricted_mana_pays_any_pip() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[oakhollow_village(), mycosynth_lattice()])
        .hand(0, &[goblin_balloon_brigade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    activate(&mut engine, PlayerId::new(0), oakhollow_village(), 1);

    cast(&mut engine, goblin_balloon_brigade()).expect("the Lattice lets green pay red");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(super::testkit::on_stack(&engine, goblin_balloon_brigade()).is_some());
}

/// **An alternative cost is priced against the same pool the spell was
/// offered from.** Cavern of Souls naming Elemental makes `{U}`, two Swamps
/// make `{B}{B}`, and Mulldrifter's evoke is `{2}{U}`. `can_cast` counted
/// the Cavern and offered the spell; the wizard's alternative-cost probe
/// read the plain pool, found `{B}{B}`, and answered "no way to cast this
/// spell".
#[test]
fn an_alternative_cost_counts_the_restricted_mana_the_spell_may_spend() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls(), swamp(), swamp()])
        .hand(0, &[mulldrifter()])
        .start();
    settle(
        &mut engine,
        baylee_core::generated::subtypes::creature::ELEMENTAL,
    );
    tap_cavern_for(&mut engine, ManaColor::Blue);
    tap_basic(&mut engine, swamp());
    tap_basic(&mut engine, swamp());
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(PlayerId::new(0)))
        .len();

    cast(&mut engine, mulldrifter()).expect("offered, so it may be taken");
    // Three mana cannot pay `{4}{U}`, so evoke is the only way there is and
    // the wizard takes it without asking. Where it does ask, it is answered.
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let evoke = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Alternative(_)))
            .expect("the evoke cost is offered: three mana, one of them the Cavern's");
        engine
            .apply(PlayerId::new(0), PlayerAction::ChooseMode(evoke))
            .expect("the evoke cost is paid");
    }
    let spell = super::testkit::on_stack(&engine, mulldrifter()).expect("on the stack");
    assert!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "the Cavern's mana paid for it"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // It resolves, is sacrificed for its evoke, and still draws two.
    pass_until(&mut engine, super::testkit::stack_is_empty);
    assert!(
        super::testkit::in_graveyard(&engine, PlayerId::new(0), mulldrifter()).is_some(),
        "evoked, so sacrificed on entering"
    );
    let hand_after = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(PlayerId::new(0)))
        .len();
    assert_eq!(hand_after, hand_before - 1 + 2, "cast one, drew two");
}

/// **The rider lands even with other mana floating.** Cavern of Souls
/// naming Bird makes `{R}` this time — no pip of Sea Eagle's — beside an
/// Island and a Swamp. Three mana pay `{1}{U}` two ways, and only the one
/// that spends the Cavern's unit on the `{1}` makes the spell uncounterable.
/// The solver prefers the admitted units wherever it has a choice; without
/// that, `W-U-B-R-G-C` order spends the Swamp's black first and the Cavern
/// stays in the pool.
#[test]
fn a_caverns_rider_lands_when_surplus_mana_floats_beside_it() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[cavern_of_souls(), island(), swamp()])
        .hand(0, &[sea_eagle()])
        .start();
    settle(
        &mut engine,
        baylee_core::generated::subtypes::creature::BIRD,
    );
    tap_cavern_for(&mut engine, ManaColor::Red);
    tap_basic(&mut engine, island());
    tap_basic(&mut engine, swamp());

    cast(&mut engine, sea_eagle()).expect("{1}{U} out of three mana");
    let spell = super::testkit::on_stack(&engine, sea_eagle()).expect("on the stack");
    assert!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable),
        "the Cavern's red paid the {{1}}"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the Swamp's black floats"
    );
    assert_eq!(restricted_total(&engine), 0);
}

/// **A spell restriction pays no activation.** Mishra's Workshop's mana is
/// for artifact *spells*, and Henge Guardian is an artifact — whose `{2}`
/// is an ability, not a spell (CR 106.6).
#[test]
fn a_spells_only_restriction_pays_no_activation() {
    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[mishras_workshop(), henge_guardian()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    let guardian = land_object(&engine, henge_guardian());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    assert!(
        !legal.abilities.contains(&(guardian, 0)),
        "nothing floats yet, so {{2}} is not offered"
    );
    activate(&mut engine, PlayerId::new(0), mishras_workshop(), 0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    assert_eq!(restricted_total(&engine), 3);
    assert!(
        !legal.abilities.contains(&(guardian, 0)),
        "three colourless for artifact spells buy the Guardian no trample: {:?}",
        legal.abilities
    );
}
