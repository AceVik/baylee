//! Creatures of the Arabian Nights / Antiquities / Legends pass (45c5bbd7),
//! part A: vanilla bodies, keywords, evasion, protection, defenders.
//! Each test plays the Oracle sentence, not the implementation.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

/// What a permanent's upkeep "unless you pay" needs, by card.
fn upkeep_mana(card: CardIndex) -> Vec<(ManaColor, u32)> {
    use ManaColor::{Black, Blue, Colorless, Green, Red, White};
    if card == ids::cosmic_horror() {
        vec![(Colorless, 3), (Black, 3)]
    } else if card == ids::arcades_sabboth() {
        vec![(Green, 1), (White, 1), (Blue, 1)]
    } else if card == ids::nicol_bolas() {
        vec![(Blue, 1), (Black, 1), (Red, 1)]
    } else if card == ids::palladia_mors() {
        vec![(Red, 1), (Green, 1), (White, 1)]
    } else if card == ids::vaevictis_asmadi() {
        vec![(Black, 1), (Red, 1), (Green, 1)]
    } else if card == ids::primordial_ooze() {
        vec![(Colorless, 1)]
    } else {
        vec![]
    }
}

/// An upkeep answer that pays what the card asks for.
fn paying(mana: Vec<(ManaColor, u32)>) -> impl FnMut(&mut Engine<RegistryLookup>) {
    move |e| {
        float(e, P0, &mana);
        yes_no(e, true);
    }
}

fn colors_of(e: &Engine<RegistryLookup>, id: ObjectId) -> ColorSet {
    e.state().object(id).unwrap().characteristics().colors
}

fn legendary(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .unwrap()
        .characteristics()
        .supertypes
        .contains(SupertypeSet::LEGENDARY)
}

fn has(e: &Engine<RegistryLookup>, id: ObjectId, k: KeywordSet) -> bool {
    keywords(e, id).contains(k)
}

/// Casts `spell` from p0's hand off exactly `mana` and returns the objects it
/// offers as targets.
#[track_caller]
fn targets_offered(
    e: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    mana: &[(ManaColor, u32)],
) -> Vec<ObjectId> {
    float(e, P0, mana);
    cast_with_floating(e, P0, spell);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected a target choice, got {:?}", e.pending());
    };
    options
}

/// The vanilla and nearly vanilla bodies: cast for exactly the printed cost,
/// they arrive with the printed colours, legendary status and size.
#[test]
fn the_plain_bodies_cost_what_they_print_and_arrive_the_size_they_print() {
    use Color::{Black, Blue, Green, Red, White};
    let rows: &[(CardIndex, &[(ManaColor, u32)], i16, i16, bool, &[Color])] = &[
        (
            ids::barbary_apes(),
            &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
            2,
            2,
            false,
            &[Green],
        ),
        (
            ids::headless_horseman(),
            &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
            2,
            2,
            false,
            &[Black],
        ),
        (
            ids::keepers_of_the_faith(),
            &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
            2,
            3,
            false,
            &[White],
        ),
        (
            ids::raging_bull(),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
            2,
            2,
            false,
            &[Red],
        ),
        (
            ids::durkwood_boars(),
            &[(ManaColor::Colorless, 4), (ManaColor::Green, 1)],
            4,
            4,
            false,
            &[Green],
        ),
        (
            ids::jasmine_boreal(),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::Green, 1),
                (ManaColor::White, 1),
            ],
            4,
            5,
            true,
            &[Green, White],
        ),
        (
            ids::moss_monster(),
            &[(ManaColor::Colorless, 3), (ManaColor::Green, 2)],
            3,
            6,
            false,
            &[Green],
        ),
        (
            ids::tobias_andrion(),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::White, 1),
                (ManaColor::Blue, 1),
            ],
            4,
            4,
            true,
            &[White, Blue],
        ),
        (
            ids::jerrard_of_the_closed_fist(),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::Red, 1),
                (ManaColor::Green, 2),
            ],
            6,
            5,
            true,
            &[Red, Green],
        ),
        (
            ids::kasimir_the_lone_wolf(),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::White, 1),
                (ManaColor::Blue, 1),
            ],
            5,
            3,
            true,
            &[White, Blue],
        ),
        (
            ids::sir_shandlar_of_eberyn(),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::Green, 1),
                (ManaColor::White, 1),
            ],
            4,
            7,
            true,
            &[Green, White],
        ),
        (
            ids::the_lady_of_the_mountain(),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::Red, 1),
                (ManaColor::Green, 1),
            ],
            5,
            5,
            true,
            &[Red, Green],
        ),
        (
            ids::torsten_von_ursus(),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::Green, 2),
                (ManaColor::White, 1),
            ],
            5,
            5,
            true,
            &[Green, White],
        ),
        (
            ids::barktooth_warbeard(),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::Black, 1),
                (ManaColor::Red, 2),
            ],
            6,
            5,
            true,
            &[Black, Red],
        ),
        (
            ids::jedit_ojanen(),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::White, 2),
                (ManaColor::Blue, 1),
            ],
            5,
            5,
            true,
            &[White, Blue],
        ),
        (
            ids::lady_orca(),
            &[
                (ManaColor::Colorless, 5),
                (ManaColor::Black, 1),
                (ManaColor::Red, 1),
            ],
            7,
            4,
            true,
            &[Black, Red],
        ),
        (
            ids::sivitri_scarzam(),
            &[
                (ManaColor::Colorless, 5),
                (ManaColor::Blue, 1),
                (ManaColor::Black, 1),
            ],
            6,
            4,
            true,
            &[Blue, Black],
        ),
    ];
    for (n, (card, cost, power, toughness, legend, colors)) in rows.iter().enumerate() {
        let mut e = game(1000 + n as u64, &[], &[], &[*card], &[]);
        float(&mut e, P0, cost);
        cast_with_floating(&mut e, P0, *card);
        settle(&mut e);
        let id = on_battlefield(&e, P0, *card)
            .unwrap_or_else(|| panic!("row {n}: the creature never arrived"));
        assert_eq!(pool_total(&e, P0), 0, "row {n}: the whole cost was spent");
        assert_eq!(pt(&e, id), (*power, *toughness), "row {n}");
        assert_eq!(legendary(&e, id), *legend, "row {n}: legendary");
        assert_eq!(
            colors_of(&e, id),
            ColorSet::from_slice(colors),
            "row {n}: colours"
        );
    }
}

/// Crimson Kobolds and Crookshank Kobolds are red creatures: Scryfall lists
/// their colour as red although the printed cost {0} is colourless (the
/// frame carries the colour). They cost nothing and arrive 0/1.
#[test]
#[ignore = "defect: Crimson/Crookshank Kobolds arrive colourless; Scryfall colours are R (no colour indicator on the card)"]
fn the_kobolds_are_red() {
    for (n, card) in [ids::crimson_kobolds(), ids::crookshank_kobolds()]
        .into_iter()
        .enumerate()
    {
        let mut e = game(990 + n as u64, &[], &[], &[card], &[]);
        cast_with_floating(&mut e, P0, card);
        settle(&mut e);
        let id = obj(&e, P0, card);
        assert_eq!(pt(&e, id), (0, 1));
        assert_eq!(colors_of(&e, id), ColorSet::from_slice(&[Color::Red]));
    }
}

/// Flying: it "can't be blocked except by creatures with flying or reach".
#[test]
fn the_fliers_are_blocked_only_by_fliers() {
    let rows = [
        (ids::vampire_bats(), ids::crimson_manticore()),
        (ids::crimson_manticore(), ids::vampire_bats()),
        (ids::arcades_sabboth(), ids::vampire_bats()),
        (ids::nicol_bolas(), ids::vampire_bats()),
        (ids::palladia_mors(), ids::vampire_bats()),
        (ids::vaevictis_asmadi(), ids::vampire_bats()),
    ];
    for (n, (flier, other_flier)) in rows.into_iter().enumerate() {
        let mut e = game_with(
            1100 + n as u64,
            &[flier],
            &[ids::barbary_apes(), other_flier],
            &[],
            &[],
            paying(upkeep_mana(flier)),
        );
        let attacker = obj(&e, P0, flier);
        assert!(has(&e, attacker, KeywordSet::FLYING), "row {n}");
        let offer = attack(&mut e, &[attacker]);
        let apes = obj(&e, P1, ids::barbary_apes());
        let flier2 = obj(&e, P1, other_flier);
        assert!(
            !may_block(&offer, apes, attacker),
            "row {n}: a ground creature blocks a flier"
        );
        assert!(
            may_block(&offer, flier2, attacker),
            "row {n}: a flier may block it"
        );
    }
}

/// The five landwalkers: "can't be blocked as long as defending player
/// controls a <land type>".
#[test]
fn the_landwalkers_are_unblockable_only_past_their_land() {
    let rows = [
        (ids::cat_warriors(), forest()),
        (ids::devouring_deep(), island()),
        (ids::segovian_leviathan(), island()),
        (ids::lost_soul(), swamp()),
        (ids::sol_kanar_the_swamp_king(), swamp()),
        (ids::mountain_yeti(), mountain()),
        (ids::righteous_avengers(), plains()),
    ];
    for (n, (walker, land)) in rows.into_iter().enumerate() {
        for (with_land, seed) in [(false, 0), (true, 1)] {
            let defenders: Vec<CardIndex> = if with_land {
                vec![ids::barbary_apes(), land]
            } else {
                vec![ids::barbary_apes()]
            };
            let mut e = game(1200 + 2 * n as u64 + seed, &[walker], &defenders, &[], &[]);
            let attacker = obj(&e, P0, walker);
            let offer = attack(&mut e, &[attacker]);
            let apes = obj(&e, P1, ids::barbary_apes());
            assert_eq!(
                may_block(&offer, apes, attacker),
                !with_land,
                "row {n} with the land: {with_land}"
            );
        }
    }
}

/// Palladia-Mors: "Flying, trample" — over a chump blocker the rest goes to
/// the player. (Flying blocker so the block is legal.)
#[test]
fn palladia_mors_tramples_over_a_flying_blocker() {
    let mut e = game_with(
        1250,
        &[ids::palladia_mors()],
        &[ids::vampire_bats()],
        &[],
        &[],
        paying(upkeep_mana(ids::palladia_mors())),
    );
    let mors = obj(&e, P0, ids::palladia_mors());
    attack(&mut e, &[mors]);
    let bats = obj(&e, P1, ids::vampire_bats());
    block(&mut e, &[(bats, mors)]);
    through_combat(&mut e);
    assert_eq!(
        life(&e, P1),
        20 - 6,
        "7 power, 1 toughness blocker: 6 trample over"
    );
    assert!(in_graveyard(&e, P1, ids::vampire_bats()).is_some());
}

/// First strike (Cosmic Horror, Gosta Dirk): the blocker dies before it
/// deals damage.
#[test]
fn first_strikers_kill_their_blocker_before_it_strikes_back() {
    for (n, striker) in [ids::cosmic_horror(), ids::gosta_dirk()]
        .into_iter()
        .enumerate()
    {
        let mut e = game_with(
            1300 + n as u64,
            &[striker],
            &[ids::keepers_of_the_faith()],
            &[],
            &[],
            paying(upkeep_mana(striker)),
        );
        let me = obj(&e, P0, striker);
        assert!(has(&e, me, KeywordSet::FIRST_STRIKE));
        attack(&mut e, &[me]);
        let keepers = obj(&e, P1, ids::keepers_of_the_faith());
        block(&mut e, &[(keepers, me)]);
        through_combat(&mut e);
        assert!(
            in_graveyard(&e, P1, ids::keepers_of_the_faith()).is_some(),
            "row {n}"
        );
        assert_eq!(
            damage_on(&e, me),
            0,
            "row {n}: it was dead before dealing damage"
        );
    }
}

/// Protection from red (Beasts of Bogardan, Ivory Guardians), from white
/// (Mountain Yeti), from black (Wall of Light): the colour cannot target it.
#[test]
fn protection_keeps_its_colour_from_targeting() {
    let bolt = lightning_bolt();
    for (n, (body, spell, mana)) in [
        (ids::beasts_of_bogardan(), bolt, (ManaColor::Red, 1u32)),
        (ids::ivory_guardians(), bolt, (ManaColor::Red, 1)),
        (ids::mountain_yeti(), path_to_exile(), (ManaColor::White, 1)),
        (ids::wall_of_light(), terror(), (ManaColor::Black, 1)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = game(
            1400 + n as u64,
            &[],
            &[body, ids::barbary_apes()],
            &[spell],
            &[],
        );
        let mana = if spell == terror() {
            vec![(ManaColor::Colorless, 1), mana]
        } else {
            vec![mana]
        };
        let options = targets_offered(&mut e, spell, &mana);
        let protected = obj(&e, P1, body);
        let apes = obj(&e, P1, ids::barbary_apes());
        assert!(
            options.contains(&apes),
            "row {n}: an unprotected creature is a target"
        );
        assert!(
            !options.contains(&protected),
            "row {n}: protection from the spell's colour"
        );
    }
}

/// Beasts of Bogardan: "gets +1/+1 as long as an opponent controls a nontoken
/// white permanent."
#[test]
fn beasts_of_bogardan_grow_next_to_an_opposing_white_permanent() {
    let e = game(1410, &[ids::beasts_of_bogardan()], &[], &[], &[]);
    let beasts = obj(&e, P0, ids::beasts_of_bogardan());
    assert_eq!(pt(&e, beasts), (3, 3));
    let e = game(
        1411,
        &[ids::beasts_of_bogardan()],
        &[ids::keepers_of_the_faith()],
        &[],
        &[],
    );
    let beasts = obj(&e, P0, ids::beasts_of_bogardan());
    assert_eq!(pt(&e, beasts), (4, 4));
    // A white permanent of its own controller does not count.
    let e2 = game(
        1412,
        &[ids::beasts_of_bogardan(), ids::keepers_of_the_faith()],
        &[],
        &[],
        &[],
    );
    assert_eq!(pt(&e2, obj(&e2, P0, ids::beasts_of_bogardan())), (3, 3));
}

/// Ivory Guardians: "Creatures named Ivory Guardians get +1/+1 as long as an
/// opponent controls a nontoken red permanent."
#[test]
fn ivory_guardians_grow_next_to_an_opposing_red_permanent() {
    let e = game(1420, &[ids::ivory_guardians()], &[], &[], &[]);
    assert_eq!(pt(&e, obj(&e, P0, ids::ivory_guardians())), (3, 3));
    let e = game(
        1421,
        &[ids::ivory_guardians()],
        &[ids::raging_bull()],
        &[],
        &[],
    );
    assert_eq!(pt(&e, obj(&e, P0, ids::ivory_guardians())), (4, 4));
    let e = game(
        1422,
        &[ids::ivory_guardians(), ids::raging_bull()],
        &[],
        &[],
        &[],
    );
    assert_eq!(pt(&e, obj(&e, P0, ids::ivory_guardians())), (3, 3));
}

/// Defender: Wall of Light, Wall of Wonder, Wall of Opposition cannot attack.
#[test]
fn the_walls_cannot_attack() {
    for (n, wall) in [
        ids::wall_of_light(),
        ids::wall_of_wonder(),
        ids::wall_of_opposition(),
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = game(1500 + n as u64, &[wall, ids::barbary_apes()], &[], &[], &[]);
        let able = attackers_offered(&mut e);
        assert!(able.contains(&obj(&e, P0, ids::barbary_apes())), "row {n}");
        assert!(
            !able.contains(&obj(&e, P0, wall)),
            "row {n}: defender attacked"
        );
    }
}

/// Wall of Wonder: "{2}{U}{U}: This creature gets +4/-4 until end of turn and
/// can attack this turn as though it didn't have defender."
#[test]
fn wall_of_wonder_pumps_and_may_then_attack() {
    let mut e = game(1510, &[ids::wall_of_wonder()], &[], &[], &[]);
    let wall = obj(&e, P0, ids::wall_of_wonder());
    assert_eq!(pt(&e, wall), (1, 5));
    float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
    );
    use_ability(&mut e, P0, ids::wall_of_wonder(), 0, &[], &[]);
    assert_eq!(pt(&e, wall), (5, 1));
    assert!(
        attackers_offered(&mut e).contains(&wall),
        "it may attack this turn"
    );
}

/// Wall of Opposition: "{1}: This creature gets +1/+0 until end of turn."
#[test]
fn wall_of_opposition_gains_a_power_for_each_mana() {
    let mut e = game(1520, &[ids::wall_of_opposition()], &[], &[], &[]);
    let wall = obj(&e, P0, ids::wall_of_opposition());
    assert_eq!(pt(&e, wall), (0, 6));
    float(&mut e, P0, &[(ManaColor::Colorless, 3)]);
    for expected in [1, 2, 3] {
        use_ability(&mut e, P0, ids::wall_of_opposition(), 0, &[], &[]);
        assert_eq!(pt(&e, wall), (expected, 6));
    }
}

/// Evil Eye of Orms-by-Gore: "Non-Eye creatures you control can't attack.
/// This creature can't be blocked except by Walls."
#[test]
fn evil_eye_attacks_alone_and_only_walls_block_it() {
    let mut e = game(
        1530,
        &[ids::evil_eye_of_orms_by_gore(), ids::barbary_apes()],
        &[ids::wall_of_light(), ids::moss_monster()],
        &[],
        &[],
    );
    let eye = obj(&e, P0, ids::evil_eye_of_orms_by_gore());
    let able = attackers_offered(&mut e);
    assert!(able.contains(&eye));
    assert!(
        !able.contains(&obj(&e, P0, ids::barbary_apes())),
        "a non-Eye cannot attack"
    );
    let offer = declare(&mut e, &[eye]);
    assert!(may_block(&offer, obj(&e, P1, ids::wall_of_light()), eye));
    assert!(!may_block(&offer, obj(&e, P1, ids::moss_monster()), eye));
}

/// Akron Legionnaire: "Except for creatures named Akron Legionnaire and
/// artifact creatures, creatures you control can't attack."
#[test]
fn akron_legionnaire_lets_only_itself_and_artifact_creatures_attack() {
    let mut e = game(
        1540,
        &[
            ids::akron_legionnaire(),
            ids::barbary_apes(),
            card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0"),
        ],
        &[],
        &[],
        &[],
    );
    let able = attackers_offered(&mut e);
    assert!(able.contains(&obj(&e, P0, ids::akron_legionnaire())));
    assert!(
        able.contains(&obj(
            &e,
            P0,
            card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
        )),
        "artifact creature (Clockwork Beast) may attack"
    );
    assert!(!able.contains(&obj(&e, P0, ids::barbary_apes())));
}

/// Elven Riders: "can't be blocked except by Walls and/or creatures with
/// flying."
#[test]
fn elven_riders_are_blocked_only_by_walls_and_fliers() {
    let mut e = game(
        1550,
        &[ids::elven_riders()],
        &[
            ids::barbary_apes(),
            ids::wall_of_light(),
            ids::vampire_bats(),
        ],
        &[],
        &[],
    );
    let riders = obj(&e, P0, ids::elven_riders());
    let offer = attack(&mut e, &[riders]);
    assert!(!may_block(&offer, obj(&e, P1, ids::barbary_apes()), riders));
    assert!(may_block(&offer, obj(&e, P1, ids::wall_of_light()), riders));
    assert!(may_block(&offer, obj(&e, P1, ids::vampire_bats()), riders));
}

/// Amrou Kithkin: "can't be blocked by creatures with power 3 or greater."
#[test]
fn amrou_kithkin_is_not_blocked_by_power_three_or_more() {
    let mut e = game(
        1560,
        &[ids::amrou_kithkin()],
        &[ids::moss_monster(), ids::barbary_apes()],
        &[],
        &[],
    );
    let kithkin = obj(&e, P0, ids::amrou_kithkin());
    let offer = attack(&mut e, &[kithkin]);
    assert!(
        !may_block(&offer, obj(&e, P1, ids::moss_monster()), kithkin),
        "power 3"
    );
    assert!(
        may_block(&offer, obj(&e, P1, ids::barbary_apes()), kithkin),
        "power 2"
    );
}

/// Answers an upkeep "sacrifice a ..." cost with the first permanent offered
/// (or with nothing).
fn sacrificing(take: bool) -> impl FnMut(&mut Engine<RegistryLookup>) {
    move |e| {
        let Pending::ChooseCards {
            player, options, ..
        } = e.pending().clone()
        else {
            panic!("a sacrifice choice was expected, got {:?}", e.pending());
        };
        let objects = if take { vec![options[0]] } else { vec![] };
        e.apply(player, PlayerAction::ChooseObjects { objects })
            .unwrap();
    }
}

/// Elder Spawn's last sentence: "can't be blocked by red creatures."
#[test]
fn elder_spawn_is_not_blocked_by_red_creatures() {
    let mut e = game_with(
        1570,
        &[ids::elder_spawn(), island()],
        &[ids::raging_bull(), ids::barbary_apes()],
        &[],
        &[],
        sacrificing(true),
    );
    let spawn = obj(&e, P0, ids::elder_spawn());
    let offer = attack(&mut e, &[spawn]);
    assert!(!may_block(&offer, obj(&e, P1, ids::raging_bull()), spawn));
    assert!(may_block(&offer, obj(&e, P1, ids::barbary_apes()), spawn));
}

/// Elder Spawn: "At the beginning of your upkeep, unless you sacrifice an
/// Island, sacrifice this creature and it deals 6 damage to you." Paying
/// with an Island keeps it, and costs the Island.
#[test]
fn elder_spawn_eats_an_island_to_stay() {
    let e = game_with(
        1571,
        &[ids::elder_spawn(), island()],
        &[],
        &[],
        &[],
        sacrificing(true),
    );
    assert!(on_battlefield(&e, P0, ids::elder_spawn()).is_some());
    assert!(
        on_battlefield(&e, P0, island()).is_none(),
        "the Island was sacrificed"
    );
    assert_eq!(life(&e, P0), 20);
}

/// Declining the Island sacrifices the Spawn and it deals 6 damage to you.
#[test]
fn elder_spawn_without_its_island_sacrifices_itself_and_hurts() {
    let e = game_with(
        1572,
        &[ids::elder_spawn(), island()],
        &[],
        &[],
        &[],
        sacrificing(false),
    );
    assert!(on_battlefield(&e, P0, ids::elder_spawn()).is_none());
    assert!(in_graveyard(&e, P0, ids::elder_spawn()).is_some());
    assert!(
        on_battlefield(&e, P0, island()).is_some(),
        "the Island stayed"
    );
    assert_eq!(life(&e, P0), 14);
}

/// With no Island to sacrifice there is nothing to decide.
#[test]
fn elder_spawn_with_no_island_is_sacrificed() {
    let e = game(1573, &[ids::elder_spawn()], &[], &[], &[]);
    assert!(in_graveyard(&e, P0, ids::elder_spawn()).is_some());
    assert_eq!(life(&e, P0), 14);
}

/// Primordial Ooze: "This creature attacks each combat if able."
#[test]
fn primordial_ooze_must_attack() {
    let mut e = game_with(
        1580,
        &[ids::primordial_ooze()],
        &[],
        &[],
        &[],
        paying(upkeep_mana(ids::primordial_ooze())),
    );
    let ooze = obj(&e, P0, ids::primordial_ooze());
    attackers_offered(&mut e);
    let refused = e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] });
    assert!(refused.is_err(), "declaring no attackers must be refused");
    declare(&mut e, &[ooze]);
}

/// Dakkon Blackblade: "power and toughness are each equal to the number of
/// lands you control."
#[test]
fn dakkon_blackblade_is_as_big_as_your_land_count() {
    let e = game(1590, &[ids::dakkon_blackblade(), forest()], &[], &[], &[]);
    assert_eq!(pt(&e, obj(&e, P0, ids::dakkon_blackblade())), (1, 1));
    let e = game(
        1591,
        &[
            ids::dakkon_blackblade(),
            forest(),
            plains(),
            island(),
            island(),
        ],
        &[forest(), forest()],
        &[],
        &[],
    );
    assert_eq!(
        pt(&e, obj(&e, P0, ids::dakkon_blackblade())),
        (4, 4),
        "four lands of mine, the opponent's two do not count"
    );
}

/// Jacques le Vert: "Green creatures you control get +0/+2."
#[test]
fn jacques_le_vert_toughens_my_green_creatures() {
    let e = game(
        1600,
        &[
            ids::jacques_le_vert(),
            ids::barbary_apes(),
            ids::raging_bull(),
        ],
        &[ids::moss_monster()],
        &[],
        &[],
    );
    assert_eq!(
        pt(&e, obj(&e, P0, ids::barbary_apes())),
        (2, 4),
        "green, mine"
    );
    assert_eq!(pt(&e, obj(&e, P0, ids::raging_bull())), (2, 2), "red");
    assert_eq!(
        pt(&e, obj(&e, P1, ids::moss_monster())),
        (3, 6),
        "green, theirs"
    );
}

/// Rabid Wombat: "Vigilance."
#[test]
fn rabid_wombat_attacks_without_tapping() {
    let mut e = game(1610, &[ids::rabid_wombat()], &[], &[], &[]);
    let wombat = obj(&e, P0, ids::rabid_wombat());
    assert_eq!(pt(&e, wombat), (0, 1));
    attack(&mut e, &[wombat]);
    assert!(!tapped(&e, wombat), "vigilance");
}

/// Rabid Wombat: "This creature gets +2/+2 for each Aura attached to it."
#[test]
#[ignore = "defect: Rabid Wombat's count filter is AttachedToBySource (what the Wombat itself is attached to), so an Aura on it adds nothing"]
fn rabid_wombat_grows_with_each_aura() {
    let mut e = game(1611, &[ids::rabid_wombat()], &[], &[ids::seeker()], &[]);
    let wombat = obj(&e, P0, ids::rabid_wombat());
    float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_at(&mut e, P0, ids::seeker(), &[wombat], &[]);
    assert_eq!(pt(&e, wombat), (2, 3), "one Aura");
}

/// The Kobolds cost nothing and arrive 0/1.
#[test]
fn the_kobolds_cost_nothing_and_are_zero_one() {
    for (n, card) in [ids::crimson_kobolds(), ids::crookshank_kobolds()]
        .into_iter()
        .enumerate()
    {
        let mut e = game(980 + n as u64, &[], &[], &[card], &[]);
        cast_with_floating(&mut e, P0, card);
        settle(&mut e);
        assert_eq!(pt(&e, obj(&e, P0, card)), (0, 1));
    }
}

/// Aisling Leprechaun: "Whenever this creature blocks or becomes blocked by
/// a creature, that creature becomes green. (This effect lasts
/// indefinitely.)"
#[test]
fn aisling_leprechaun_turns_what_it_fights_green() {
    let mut e = game(
        1620,
        &[ids::aisling_leprechaun()],
        &[ids::raging_bull()],
        &[],
        &[],
    );
    let lep = obj(&e, P0, ids::aisling_leprechaun());
    let bull = obj(&e, P1, ids::raging_bull());
    attack(&mut e, &[lep]);
    block(&mut e, &[(bull, lep)]);
    settle(&mut e);
    assert_eq!(colors_of(&e, bull), ColorSet::from_slice(&[Color::Green]));
    // Indefinitely: still green two turns on.
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    assert_eq!(colors_of(&e, bull), ColorSet::from_slice(&[Color::Green]));
}

/// The same when the Leprechaun is the blocker.
#[test]
fn aisling_leprechaun_blocking_turns_the_attacker_green() {
    let mut e = game(
        1621,
        &[ids::raging_bull()],
        &[ids::aisling_leprechaun()],
        &[],
        &[],
    );
    let bull = obj(&e, P0, ids::raging_bull());
    let lep = obj(&e, P1, ids::aisling_leprechaun());
    attack(&mut e, &[bull]);
    block(&mut e, &[(lep, bull)]);
    settle(&mut e);
    assert_eq!(colors_of(&e, bull), ColorSet::from_slice(&[Color::Green]));
}
