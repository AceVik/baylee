//! `cards/enchantments/mv_2/path_of_mettle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "ddb021df-ae4a-4ac1-8353-d0b375761714": first strike.
fn white_knight() -> CardIndex {
    card_index("ddb021df-ae4a-4ac1-8353-d0b375761714")
}

// oracle_id = "30997b43-fc13-41d3-8064-1ccc2cb6fd2b": haste.
fn raging_goblin() -> CardIndex {
    card_index("30997b43-fc13-41d3-8064-1ccc2cb6fd2b")
}

/// Path of Mettle prints "When this enchantment enters, it deals 1 damage to
/// each creature that doesn't have first strike, double strike, vigilance, or
/// haste", and the board is three identical printed 1/1 Llanowar Elves — two
/// of mine and one across the table — of which exactly one has been given
/// haste by Lightning Greaves before the enchantment arrives. The Greaves
/// equip for `{0}`, so nothing about the mana spent on the enchantment is
/// confounded by them, and both halves of the sentence get a witness of the
/// same body: one Elf dies beside the survivor of my own, and one dies on the
/// other side of the table, because "each creature" names no controller.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn path_of_mettle_spares_the_creature_that_has_one_of_its_four_keywords() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
                lightning_greaves(),
            ],
        )
        .hand(0, &[path_of_mettle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (armed, bare) = (elves[0], elves[1]);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "an Elf across the table"
    );
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are out");
    assert!(
        !keywords(&engine, armed).contains(KeywordSet::HASTE),
        "nothing is equipped yet, so nothing has haste"
    );

    // Equip {0} (CR 702.6). Its whole price is the tap symbol, so the pool the
    // enchantment below is paid out of is exactly what the four lands hold —
    // and the ability index is read out of the offer rather than guessed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == greaves)
        .expect("Equip {0} is the only activated ability the Greaves print");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("equip costs its own tap symbol and no mana at all");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options.len(),
        2,
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        options.contains(&armed) && options.contains(&bare),
        "and the Elf across the table is not one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![armed],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(armed))
    });

    let granted = keywords(&engine, armed);
    assert!(
        granted.contains(KeywordSet::HASTE),
        "the Greaves grant haste, the fourth of the four keywords the trigger \
         names: {granted:?}"
    );
    assert_eq!(
        pt(&engine, armed),
        (1, 1),
        "and they change no body, so it is still the printed 1/1 the bare Elf is"
    );

    // The Elves are named as the printing kept back, because they are the
    // creatures this test reads afterwards: what is tapped is the two Plains
    // and the two Mountains, which is {W}{W}{R}{R} for a {R}{W}.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Mountains, and neither Elf tapped for anything"
    );
    cast_front_face(&mut engine, p0, path_of_mettle());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, path_of_mettle()).is_some(),
        "the enchantment resolved onto the table"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![armed],
        "1 damage on a printed 1/1 is lethal (CR 704.5g), so the only Elf of \
         mine still standing is the one the trigger's filter excluded"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the bare Elf the trigger did not exclude is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"each creature\" names no controller, so the Elf across the table \
         died to the same resolution"
    );
    assert!(
        keywords(&engine, armed).contains(KeywordSet::HASTE),
        "and the survivor kept the keyword that spared it"
    );
}

/// "When Path of Mettle enters, it deals 1 damage to each creature that doesn't
/// have first strike, double strike, vigilance, or haste."
///
/// Ruling 2018-01-19: "It doesn't deal 1 damage to each creature for each
/// ability that creature doesn't have." The board holds a creature with each
/// of three keywords the other test does not play — White Knight (first
/// strike), Skyhunter Skirmisher (double strike) and Serra Angel (vigilance) —
/// beside a bare Gray Ogre of each side. The three keyword creatures have no
/// damage marked; each Ogre has exactly 1.
#[test]
fn path_of_mettle_spares_first_strike_double_strike_and_vigilance() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                mountain(),
                white_knight(),
                skyhunter_skirmisher(),
                serra_angel(),
                gray_ogre(),
            ],
        )
        .hand(0, &[path_of_mettle()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let knight = on_battlefield(&engine, p0, white_knight()).expect("the Knight");
    let skirmisher = on_battlefield(&engine, p0, skyhunter_skirmisher()).expect("the Skirmisher");
    let angel = on_battlefield(&engine, p0, serra_angel()).expect("the Angel");
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, path_of_mettle());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, path_of_mettle()).is_some(),
        "Path of Mettle resolved"
    );
    assert_eq!(kit::damage_on(&engine, knight), 0, "first strike: spared");
    assert!(
        kit::is_on_battlefield(&engine, skirmisher) && kit::damage_on(&engine, skirmisher) == 0,
        "double strike: spared"
    );
    assert_eq!(kit::damage_on(&engine, angel), 0, "vigilance: spared");
    assert_eq!(
        kit::damage_on(&engine, mine),
        1,
        "a creature with none of the four takes 1 damage, once"
    );
    assert_eq!(kit::damage_on(&engine, theirs), 1, "whoever controls it");
}

/// A table with Path of Mettle and `attackers` on player 0's battlefield,
/// player 0 attacking player 1 with every one of them. Returns the face
/// Path of Mettle shows once the attack trigger has resolved: 0 the
/// enchantment, 1 the land.
#[track_caller]
fn the_face_of_the_path_after_attacking_with(attackers: &[CardIndex]) -> u8 {
    let p0 = kit::P0;
    let mut board = vec![path_of_mettle()];
    board.extend_from_slice(attackers);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let path = on_battlefield(&engine, p0, path_of_mettle()).expect("Path of Mettle");
    let mut creatures = Vec::new();
    for card in attackers {
        let found = all_on_battlefield(&engine, p0, *card);
        creatures.push(
            *found
                .iter()
                .find(|id| !creatures.contains(*id))
                .expect("the attacker is on the battlefield"),
        );
    }

    kit::declare_attack(&mut engine, &creatures, kit::P1);
    pass_until(&mut engine, stack_is_empty);

    kit::face_of(&engine, path)
}

/// "Whenever you attack with at least two creatures that have first strike,
/// double strike, vigilance, and/or haste, transform Path of Mettle."
///
/// Ruling 2018-01-19: "any two attacking creatures you control need to each
/// have any one of the four listed abilities. They don't need to share one of
/// those abilities." Attacking with vigilance and haste, with first strike and
/// double strike, and with two of the same, transforms it; and what it
/// transforms into is Metzali, Tower of Triumph, a legendary land and no
/// enchantment any more.
#[test]
fn path_of_mettle_transforms_when_two_attackers_each_have_one_of_the_four_keywords() {
    for (a, b) in [
        (serra_angel(), raging_goblin()),
        (white_knight(), skyhunter_skirmisher()),
        (serra_angel(), serra_angel()),
    ] {
        assert_eq!(
            the_face_of_the_path_after_attacking_with(&[a, b]),
            1,
            "two attackers with a keyword each turn the Path over"
        );
    }
}

/// The count is of attackers *that have* a keyword: one such attacker beside
/// ones without, one such attacker alone, and two without any, all leave Path
/// of Mettle an enchantment.
#[test]
fn path_of_mettle_does_not_transform_for_fewer_than_two_qualifying_attackers() {
    assert_eq!(
        the_face_of_the_path_after_attacking_with(&[serra_angel(), gray_ogre()]),
        0,
        "one qualifying attacker and one that does not"
    );
    assert_eq!(
        the_face_of_the_path_after_attacking_with(&[serra_angel()]),
        0,
        "one attacker only"
    );
    assert_eq!(
        the_face_of_the_path_after_attacking_with(&[gray_ogre(), wild_elephant()]),
        0,
        "two attackers and none of the four keywords"
    );
    assert_eq!(
        the_face_of_the_path_after_attacking_with(&[serra_angel(), raging_goblin()]),
        1,
        "the same attack with a second qualifying creature does turn it over"
    );
}

/// The other side of the transformation: the permanent that stands there is
/// Metzali, Tower of Triumph — a legendary land, no longer an enchantment.
#[test]
fn path_of_mettle_turns_into_a_legendary_land() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[path_of_mettle(), serra_angel(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let path = on_battlefield(&engine, p0, path_of_mettle()).expect("Path of Mettle");
    let before = types(&engine, path);
    assert!(
        before.contains(TypeSet::ENCHANTMENT) && !before.contains(TypeSet::LAND),
        "an enchantment to begin with"
    );
    let angels = all_on_battlefield(&engine, p0, serra_angel());

    kit::declare_attack(&mut engine, &angels, kit::P1);
    pass_until(&mut engine, stack_is_empty);

    let after = types(&engine, path);
    assert!(after.contains(TypeSet::LAND), "a land");
    assert!(!after.contains(TypeSet::ENCHANTMENT), "and no enchantment");
    assert!(
        kit::has_supertype(&engine, path, SupertypeSet::LEGENDARY),
        "legendary"
    );
    assert_eq!(
        kit::controller_of(&engine, path),
        p0,
        "the same permanent, transformed where it stands"
    );
}

/// A table with Metzali, Tower of Triumph on player 0's battlefield (turned
/// over from Path of Mettle where it stands), three Plains and two Mountains to
/// pay with, and `mine` besides; player 1 has `theirs`. Player 0 is in their
/// first main phase.
#[track_caller]
fn metzali_with(
    seats: usize,
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = kit::P0;
    let mut board = vec![
        path_of_mettle(),
        plains(),
        plains(),
        plains(),
        mountain(),
        mountain(),
    ];
    board.extend_from_slice(mine);
    let mut engine = Duel::table(SEED, forest(), seats)
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let metzali = on_battlefield(&engine, p0, path_of_mettle()).expect("the card");
    kit::turn_over(&mut engine, p0, metzali, path_of_mettle());
    assert_eq!(kit::face_of(&engine, metzali), 1, "Metzali");
    (engine, metzali)
}

/// Metzali's "{T}: Add one mana of any color."
///
/// All five colours are offered, one is chosen and exactly one mana of it is
/// made; Metzali is tapped.
#[test]
fn metzali_taps_for_one_mana_of_any_color() {
    let p0 = kit::P0;
    for colour in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let (mut engine, metzali) = metzali_with(2, &[], &[]);
        activate(&mut engine, p0, path_of_mettle(), 0);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("asked which colour: {:?}", engine.pending())
        };
        assert_eq!(options.len(), 5, "any of the five colours: {options:?}");
        engine
            .apply(p0, PlayerAction::ChooseColor(colour))
            .expect("the colour was offered");

        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "one mana");
        assert_eq!(pool.available(colour), 1, "of the colour chosen");
        assert!(is_tapped(&engine, metzali), "{{T}} is the cost");
    }
}

/// Metzali's "{1}{R}, {T}: Metzali deals 2 damage to each opponent."
///
/// At a table of three both opponents take 2 and the controller takes none;
/// Metzali is tapped and `{1}{R}` is spent.
#[test]
fn metzali_deals_two_damage_to_each_opponent() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, metzali) = metzali_with(3, &[], &[]);
    tap_mana_except(&mut engine, p0, metzali);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    activate(&mut engine, p0, path_of_mettle(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::life(&engine, p1), 18, "one opponent takes 2");
    assert_eq!(kit::life(&engine, p2), 18, "and so does the other");
    assert_eq!(kit::life(&engine, p0), 20, "the controller takes none");
    assert!(is_tapped(&engine, metzali), "{{T}} is the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "{{1}}{{R}} was spent out of five"
    );
}

/// Metzali's "{2}{W}, {T}: Choose a creature at random that attacked this
/// turn. Destroy that creature."
///
/// Player 0 attacks with one Gray Ogre and keeps a Wild Elephant at home;
/// player 1 has a Gray Ogre that did not attack. The only creature that
/// attacked is the Ogre, so it is the one that is destroyed — activated with
/// the attack declared, before blockers (ruling 2018-01-19) — and the other two
/// survive.
#[test]
fn metzali_destroys_the_creature_that_attacked() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[gray_ogre(), wild_elephant()], &[gray_ogre()]);
    let attacker = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let at_home = on_battlefield(&engine, p0, wild_elephant()).expect("my Elephant");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    kit::declare_attack(&mut engine, &[attacker], p1);
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        kit::zone_of(&engine, attacker),
        Zone::Graveyard,
        "the one creature that attacked is destroyed"
    );
    assert!(
        kit::is_on_battlefield(&engine, at_home),
        "a creature that did not attack is not a candidate"
    );
    assert!(
        kit::is_on_battlefield(&engine, theirs),
        "nor is an opponent's that did not attack"
    );
}

/// Ruling 2018-01-19: "Metzali's last ability can be activated after combat
/// damage has been dealt. You'll choose a creature at random that attacked
/// this turn and has survived combat damage." The Ogre attacks unblocked,
/// deals its damage and lives; in the second main phase Metzali destroys it.
#[test]
fn metzali_may_destroy_an_attacker_after_combat_damage() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[gray_ogre()], &[]);
    let attacker = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    kit::declare_attack(&mut engine, &[attacker], p1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        kit::life(&engine, p1),
        18,
        "the Ogre's combat damage was dealt"
    );
    assert!(kit::is_on_battlefield(&engine, attacker), "and it survived");
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        kit::zone_of(&engine, attacker),
        Zone::Graveyard,
        "it attacked this turn, so it is chosen and destroyed"
    );
}

/// "Choose a creature at random that attacked this turn": with no creature
/// having attacked there is nothing to choose and nothing is destroyed — every
/// creature on both sides stays.
#[test]
fn metzali_destroys_nothing_when_no_creature_attacked() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[gray_ogre()], &[wild_elephant()]);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, wild_elephant()).expect("their Elephant");
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::settle(&mut engine, p0, |_, _| None);

    assert!(kit::is_on_battlefield(&engine, mine), "nothing attacked");
    assert!(kit::is_on_battlefield(&engine, theirs), "nothing attacked");
}

/// "Choose a creature at random": exactly one. Two creatures attack, and one —
/// not both, not neither — is destroyed.
#[test]
fn metzali_destroys_exactly_one_of_two_attackers() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[gray_ogre(), wild_elephant()], &[]);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("my Elephant");
    kit::declare_attack(&mut engine, &[ogre, elephant], p1);
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::settle(&mut engine, p0, |_, _| None);

    let dead = [ogre, elephant]
        .iter()
        .filter(|id| kit::zone_of(&engine, **id) == Zone::Graveyard)
        .count();
    assert_eq!(dead, 1, "one of the two attackers was chosen and destroyed");
}

/// Ruling 2018-01-19: "A creature with indestructible can be chosen at random.
/// It won't be destroyed." The Darksteel Gargoyle is the only creature that
/// attacked, so it is the one chosen — and destroying it does nothing; the
/// Ogre that stayed at home is not chosen in its place.
#[test]
fn metzali_choosing_an_indestructible_attacker_destroys_nothing() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[darksteel_gargoyle(), gray_ogre()], &[]);
    let gargoyle = on_battlefield(&engine, p0, darksteel_gargoyle()).expect("the Gargoyle");
    let at_home = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    kit::declare_attack(&mut engine, &[gargoyle], p1);
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::settle(&mut engine, p0, |_, _| None);

    assert!(
        kit::is_on_battlefield(&engine, gargoyle),
        "indestructible: chosen and not destroyed"
    );
    assert!(
        kit::is_on_battlefield(&engine, at_home),
        "and the creature that did not attack was never a candidate"
    );
}

/// "Choose a creature at random that attacked this turn": any creature,
/// whoever controls it, and the ability has no timing restriction. On player
/// 1's turn their Ogre attacks player 0 while player 0's own Ogre stays home;
/// player 0 activates Metzali with the attack declared, and the creature
/// destroyed is the opponent's attacker.
#[test]
fn metzali_destroys_an_opponents_attacker_on_their_turn() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, metzali) = metzali_with(2, &[gray_ogre()], &[gray_ogre()]);
    let at_home = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let attacker = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    kit::attack_on_their_turn(&mut engine, p1, &[attacker], p0);
    tap_mana_except(&mut engine, p0, metzali);

    activate(&mut engine, p0, path_of_mettle(), 2);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert_eq!(
        kit::zone_of(&engine, attacker),
        Zone::Graveyard,
        "the only creature that attacked this turn is theirs, and it is destroyed"
    );
    assert!(
        kit::is_on_battlefield(&engine, at_home),
        "my own creature did not attack and is not a candidate"
    );
}
