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
        .battlefield(0, &[mystic_gate(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The gate's combination line costs `{1}, {T}`, and an ability whose mana
    // is not floating is not offered at all — so the Forest goes first.
    activate(&mut engine, p0, forest(), 0);
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
/// *offered*, while `spend_restricted` on the far side of the cast wizard
/// would have paid for it without complaint. Both halves ask the same
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
/// at all — the ability still resolves (CR 106.6a), it just adds nothing.
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

/// CR 500.4: "When a step or phase ends, any unused mana left in a player's
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
