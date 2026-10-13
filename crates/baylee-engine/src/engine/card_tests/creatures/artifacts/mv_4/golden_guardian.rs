//! `cards/creatures/artifacts/mv_4/golden_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::subtypes::creature;

/// `Defender` on the front face: a 4/4 artifact Golem that is declared as an
/// attacker by nobody — `young_wolf()` beside it can attack, the Guardian is
/// not offered.
#[test]
fn golden_guardian_has_defender_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[golden_guardian(), young_wolf(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guardian = on_battlefield(&engine, p0, golden_guardian()).expect("guardian on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, guardian), (4, 4));
    assert!(keywords(&engine, guardian).contains(KeywordSet::DEFENDER));
    let types = types(&engine, guardian);
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "an artifact creature"
    );
    assert!(
        kit::has_subtype(&engine, guardian, creature::GOLEM),
        "a Golem"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers, got {:?}", engine.pending());
    };
    assert!(
        attackers.contains(&wolf),
        "young wolf without defender is a legal attacker"
    );
    assert!(
        !attackers.contains(&guardian),
        "golden guardian with defender cannot be declared as an attacker"
    );
}

/// Player 0 controls Golden Guardian, two Forests to pay `{2}` with and
/// `mine`; player 1 controls `theirs`. Player 0 is in their first main phase.
#[track_caller]
fn the_guardian_with(
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = kit::P0;
    let mut board = vec![golden_guardian(), forest(), forest()];
    board.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guardian = on_battlefield(&engine, p0, golden_guardian()).expect("the Guardian");
    (engine, guardian)
}

/// Pays `{2}`, activates the fight and names `target`. Returns the menu of
/// creatures the ability offered, or `None` when it asked for no target.
#[track_caller]
fn fight(
    engine: &mut Engine<RegistryLookup>,
    guardian: ObjectId,
    target: ObjectId,
) -> Option<Vec<ObjectId>> {
    let p0 = kit::P0;
    start_the_fight(engine, guardian);
    let mut menu = None;
    kit::settle(engine, p0, |_, pending| {
        if let Pending::ChooseTargets { options, .. } = pending {
            menu = Some(options.clone());
        }
        kit::choose_targets(pending, &[target], &[])
    });
    menu
}

/// Pays `{2}` and activates the fight, leaving it waiting for its target.
#[track_caller]
fn start_the_fight(engine: &mut Engine<RegistryLookup>, guardian: ObjectId) {
    let p0 = kit::P0;
    tap_mana_except(engine, p0, guardian);
    let offered = kit::offered(engine, guardian);
    let index = *offered
        .first()
        .unwrap_or_else(|| panic!("the fight is offered with {{2}} floating: {offered:?}"));
    kit::activate_object(engine, p0, guardian, index);
}

/// "{2}: This creature fights another target creature you control."
///
/// Player 0 also controls a Gray Ogre, and player 1 a Gray Ogre of their own.
/// The target menu is exactly the Ogre player 0 controls — not the Guardian
/// itself ("another") and not the opponent's creature ("you control"). The
/// Guardian deals 4 to its Ogre, which dies, and takes 2 back, which it
/// survives.
#[test]
fn golden_guardian_fights_another_creature_you_control() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[gray_ogre()]);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");

    let menu = fight(&mut engine, guardian, mine).expect("the fight asks for a target");

    assert_eq!(
        menu,
        vec![mine],
        "another creature you control: not the Guardian itself ({guardian:?}), not the \
         opponent's Ogre ({theirs:?})"
    );
    assert_eq!(
        kit::zone_of(&engine, mine),
        Zone::Graveyard,
        "4 damage kills a 2/2"
    );
    assert!(kit::is_on_battlefield(&engine, guardian), "it survives");
    assert_eq!(kit::damage_on(&engine, guardian), 2, "having taken 2 back");
    assert!(
        kit::is_on_battlefield(&engine, theirs),
        "their Ogre took no part"
    );
}

/// "{2}: This creature fights another target creature **you control**": with
/// no other creature of its controller's, the fight has no legal target and is
/// not offered although `{2}` is floating and an opponent's creature is there
/// to fight.
#[test]
fn golden_guardian_cannot_fight_when_it_is_the_only_creature_you_control() {
    let (mut engine, guardian) = the_guardian_with(&[], &[gray_ogre()]);

    tap_mana_except(&mut engine, kit::P0, guardian);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    assert!(
        kit::offered(&engine, guardian).is_empty(),
        "no creature of mine but the Guardian itself, and an opponent's is no \
         target: nothing of the Guardian's is offered"
    );

    // The same board with a second creature of mine: now there is something to
    // fight, and the ability is offered for the same `{2}`.
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[gray_ogre()]);
    tap_mana_except(&mut engine, kit::P0, guardian);
    assert!(
        !kit::offered(&engine, guardian).is_empty(),
        "with another creature of mine the fight is offered"
    );
}

/// "When this creature dies this turn, return it to the battlefield
/// transformed under your control" — by the fight itself.
///
/// The Guardian fights an Earth Elemental, a 4/5: it takes 4 and dies, and
/// returns to the battlefield as Gold-Forge Garrison — a land and no
/// creature, no artifact, face 1 of the same card — untapped and under player
/// 0's control. The Elemental took 4 and lives.
#[test]
fn golden_guardian_that_dies_in_its_own_fight_returns_as_gold_forge_garrison() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[earth_elemental()], &[]);
    let elemental = on_battlefield(&engine, p0, earth_elemental()).expect("the Elemental");

    fight(&mut engine, guardian, elemental);

    assert!(
        kit::is_on_battlefield(&engine, elemental),
        "a 4/5 survives 4 damage"
    );
    assert_eq!(kit::damage_on(&engine, elemental), 4);
    let garrison = on_battlefield(&engine, p0, golden_guardian())
        .expect("it came back to the battlefield under its controller's control");
    assert_eq!(kit::face_of(&engine, garrison), 1, "transformed");
    let types = types(&engine, garrison);
    assert!(
        types.contains(TypeSet::LAND),
        "Gold-Forge Garrison is a land"
    );
    assert!(
        !types.contains(TypeSet::CREATURE) && !types.contains(TypeSet::ARTIFACT),
        "and nothing else: {types:?}"
    );
    assert_eq!(kit::controller_of(&engine, garrison), p0);
    assert!(
        !is_tapped(&engine, garrison),
        "it returns untapped, since the sentence does not say tapped"
    );
}

/// Ruling 2018-01-19: "Once Golden Guardian's activated ability has resolved,
/// it will return to the battlefield transformed if it dies for any reason in
/// that turn."
///
/// The fight against a Gray Ogre leaves the Guardian alive; killing it
/// afterwards, the same turn, returns it transformed.
#[test]
fn golden_guardian_that_dies_later_the_same_turn_returns_transformed() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    fight(&mut engine, guardian, ogre);
    assert!(kit::is_on_battlefield(&engine, guardian), "alive for now");

    kill(&mut engine, guardian);

    let garrison = on_battlefield(&engine, p0, golden_guardian()).expect("it came back");
    assert_eq!(kit::face_of(&engine, garrison), 1, "transformed");
    assert!(types(&engine, garrison).contains(TypeSet::LAND));
    assert_eq!(kit::controller_of(&engine, garrison), p0);
}

/// "…dies **this turn**": on a later turn the delayed trigger is gone, and the
/// Guardian stays in the graveyard.
#[test]
fn golden_guardian_that_dies_on_a_later_turn_stays_dead() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    fight(&mut engine, guardian, ogre);
    assert!(walk_to_own_main(&mut engine, p0), "player 0's next turn");
    assert!(engine.state().turn.number >= 3, "a later turn");
    assert!(
        kit::is_on_battlefield(&engine, guardian),
        "the Guardian lived through"
    );

    kill(&mut engine, guardian);

    assert!(
        on_battlefield(&engine, p0, golden_guardian()).is_none(),
        "nothing came back"
    );
    assert!(
        in_graveyard(&engine, p0, golden_guardian()).is_some(),
        "the card lies in its owner's graveyard"
    );
}

/// Ruling 2018-01-19: "If Golden Guardian leaves the battlefield before its
/// activated ability has resolved, it won't be returned to the battlefield
/// when the ability resolves", and "neither creature will deal or be dealt
/// damage" (CR 701.14b).
///
/// With the fight on the stack the Guardian is destroyed in response: the
/// Ogre takes no damage and the Guardian stays in the graveyard.
#[test]
fn golden_guardian_that_leaves_before_the_fight_resolves_is_not_returned() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    start_the_fight(&mut engine, guardian);
    kit::name_the_target(&mut engine, ogre);

    bury(&mut engine, &[guardian]);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(kit::damage_on(&engine, ogre), 0, "the Ogre was not hurt");
    assert!(kit::is_on_battlefield(&engine, ogre));
    assert!(
        on_battlefield(&engine, p0, golden_guardian()).is_none(),
        "and the Guardian was not returned"
    );
}

/// CR 608.2b and 701.14b: the fight's only target is gone when it resolves —
/// the ability does not resolve at all. The Guardian takes no damage, and it
/// has set nothing up: killed later the same turn it stays in the graveyard.
#[test]
fn golden_guardian_whose_fight_target_is_gone_takes_no_damage_and_sets_nothing_up() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    start_the_fight(&mut engine, guardian);
    kit::name_the_target(&mut engine, ogre);

    bury(&mut engine, &[ogre]);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        kit::damage_on(&engine, guardian),
        0,
        "neither creature fights or deals damage"
    );

    kill(&mut engine, guardian);

    assert!(
        on_battlefield(&engine, p0, golden_guardian()).is_none(),
        "the ability never resolved, so nothing is waiting for the Guardian to die"
    );
}

/// The back face, as it stands on the battlefield: "{T}: Add two mana of any
/// one color." — one colour chosen, two mana of it.
#[test]
fn gold_forge_garrison_taps_for_two_mana_of_one_colour() {
    let p0 = kit::P0;
    let (mut engine, guardian) = the_guardian_with(&[], &[]);
    kit::turn_over(&mut engine, p0, guardian, golden_guardian());
    assert_eq!(kit::face_of(&engine, guardian), 1, "turned over");

    let offered = kit::offered(&engine, guardian);
    assert!(
        !offered.is_empty(),
        "the Garrison's mana ability is offered: {offered:?}"
    );
    kit::activate_object(&mut engine, p0, guardian, offered[0]);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("asked which colour: {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "any one of the five colours: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 2, "two red mana");
    assert_eq!(pool.total(), 2, "and nothing else");
    assert!(is_tapped(&engine, guardian), "{{T}} is the cost");
}

/// The back face's "{4}, {T}: Create a 4/4 colorless Golem artifact creature
/// token."
///
/// The Garrison is turned over with four Forests beside it. The ability makes
/// exactly one token: a 4/4 colourless Golem that is an artifact and a
/// creature, under its controller's control.
#[test]
fn gold_forge_garrison_creates_a_four_four_colorless_golem_artifact_token() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[golden_guardian(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let garrison = on_battlefield(&engine, p0, golden_guardian()).expect("the card");
    kit::turn_over(&mut engine, p0, garrison, golden_guardian());
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no token before the ability"
    );

    tap_mana_except(&mut engine, p0, garrison);
    let offered = kit::offered(&engine, garrison);
    assert!(
        offered.len() >= 2,
        "the mana ability and the {{4}}, {{T}} ability: {offered:?}"
    );
    kit::activate_object(
        &mut engine,
        p0,
        garrison,
        *offered.last().expect("the token ability"),
    );
    kit::settle(&mut engine, p0, |_, _| None);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token: {tokens:?}");
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (4, 4), "a 4/4");
    let types = types(&engine, token);
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "an artifact creature"
    );
    assert!(kit::has_subtype(&engine, token, creature::GOLEM), "a Golem");
    assert!(kit::colors_of(&engine, token).is_empty(), "colorless");
    assert!(is_tapped(&engine, garrison), "{{T}} is part of the cost");
}
