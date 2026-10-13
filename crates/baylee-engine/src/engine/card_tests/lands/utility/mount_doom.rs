//! `cards/lands/utility/mount_doom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "14c8f55d-d177-4c25-a931-ebeb9e6062a0"
fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The index of Mount Doom's `{T}, Pay 1 life: Add {B} or {R}.`
const MANA: u32 = 0;
/// The index of its `{1}{B}{R}, {T}: Mount Doom deals 1 damage to each opponent.`
const PING: u32 = 1;
/// The index of its `{5}{B}{R}, {T}, Sacrifice Mount Doom and a legendary
/// artifact: Choose up to two creatures, then destroy the rest. Activate only
/// as a sorcery.`
const WIPE: u32 = 2;

/// The permanents of a board for Mount Doom's third ability.
struct Board {
    /// Player 0's Mount Doom.
    doom: ObjectId,
    /// Player 0's Mox Opal: the legendary artifact to sacrifice.
    opal: ObjectId,
    /// Player 0's Gray Ogre and Grizzly Bears.
    mine: [ObjectId; 2],
    /// Player 1's Gray Ogre and Wild Elephant.
    theirs: [ObjectId; 2],
}

impl Board {
    /// All four plain creatures, player 0's first.
    fn creatures(&self) -> [ObjectId; 4] {
        [self.mine[0], self.mine[1], self.theirs[0], self.theirs[1]]
    }
}

/// Player 0 controls Mount Doom, five Forests, a Swamp and a Mountain (exactly
/// `{5}{B}{R}` besides Mount Doom's own `{T}`), a Mox Opal, a Gray Ogre and a
/// Grizzly Bears; player 1 controls a Gray Ogre and a Wild Elephant. Player 0
/// is in their first main phase.
#[track_caller]
fn the_board() -> (Engine<RegistryLookup>, Board) {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mount_doom(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                mountain(),
                mox_opal(),
                gray_ogre(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[gray_ogre(), wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let board = Board {
        doom: on_battlefield(&engine, p0, mount_doom()).expect("Mount Doom"),
        opal: on_battlefield(&engine, p0, mox_opal()).expect("the Opal"),
        mine: [
            on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre"),
            on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears"),
        ],
        theirs: [
            on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre"),
            on_battlefield(&engine, p1, wild_elephant()).expect("their Elephant"),
        ],
    };
    (engine, board)
}

/// Floats `{5}{B}{R}` by tapping everything except Mount Doom, activates the
/// wipe and answers what it asks: the legendary artifact to sacrifice is the
/// Opal; the creatures to keep are `keep`. The choice of creatures must not be
/// a target. Returns the `(min, max, options)` of the creature question, or
/// `None` if it was never asked.
#[track_caller]
fn wipe_keeping(
    engine: &mut Engine<RegistryLookup>,
    board: &Board,
    keep: &[ObjectId],
) -> Option<(u8, u8, Vec<ObjectId>)> {
    let p0 = kit::P0;
    tap_mana_except(engine, p0, board.doom);
    kit::activate_object(engine, p0, board.doom, WIPE);
    let mut asked = None;
    kit::settle(engine, p0, |_, pending| match pending {
        Pending::ChooseTargets { .. } => {
            panic!("`choose up to two creatures` does not target: {pending:?}")
        }
        Pending::ChooseCards { options, .. } if options.contains(&board.opal) => {
            kit::choose_cards(pending, &[board.opal])
        }
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            asked = Some((*min, *max, options.clone()));
            kit::choose_cards(pending, keep)
        }
        _ => None,
    });
    asked
}

/// "Choose up to two creatures, then destroy the rest."
///
/// The creatures chosen are one of player 0's and one of player 1's — "any
/// creatures", whoever controls them — and the other two are destroyed. The
/// question offers every creature on the battlefield, asks for none to two,
/// and the permanents that are not creatures (the lands) are not touched.
#[test]
fn mount_doom_destroys_every_creature_except_the_two_chosen() {
    let (mut engine, board) = the_board();
    let keep = [board.mine[1], board.theirs[0]];

    let asked = wipe_keeping(&mut engine, &board, &keep).expect("the controller chooses");

    let (min, max, mut options) = asked;
    assert_eq!((min, max), (0, 2), "up to two creatures");
    options.sort_unstable();
    let mut all = board.creatures().to_vec();
    all.sort_unstable();
    assert_eq!(
        options, all,
        "every creature on the battlefield, whoever controls it, and nothing else"
    );
    assert!(
        kit::is_on_battlefield(&engine, board.mine[1]),
        "my Bears kept"
    );
    assert!(
        kit::is_on_battlefield(&engine, board.theirs[0]),
        "their Ogre kept"
    );
    assert_eq!(
        kit::zone_of(&engine, board.mine[0]),
        Zone::Graveyard,
        "my other creature is destroyed"
    );
    assert_eq!(
        kit::zone_of(&engine, board.theirs[1]),
        Zone::Graveyard,
        "their other creature is destroyed"
    );
    assert_eq!(
        all_on_battlefield(&engine, kit::P0, forest()).len(),
        5,
        "the lands are no creatures and stay"
    );
}

/// "Choose up to two": choosing none is a legal answer, and then *every*
/// creature is "the rest" and is destroyed.
#[test]
fn mount_doom_choosing_no_creature_destroys_them_all() {
    let (mut engine, board) = the_board();

    let asked = wipe_keeping(&mut engine, &board, &[]).expect("the controller chooses");

    assert_eq!(asked.0, 0, "choosing none is allowed");
    for creature in board.creatures() {
        assert_eq!(
            kit::zone_of(&engine, creature),
            Zone::Graveyard,
            "{creature:?} is destroyed"
        );
    }
}

/// "{5}{B}{R}, {T}, Sacrifice Mount Doom and a legendary artifact"
///
/// Both sacrifices are costs: Mount Doom and the Mox Opal are in their owner's
/// graveyard, and all seven mana are spent, once the ability has been
/// activated and has resolved.
#[test]
fn mount_doom_sacrifices_itself_and_a_legendary_artifact_as_its_cost() {
    let (mut engine, board) = the_board();

    wipe_keeping(&mut engine, &board, &[]);

    assert_eq!(
        kit::zone_of(&engine, board.doom),
        Zone::Graveyard,
        "Mount Doom is sacrificed"
    );
    assert_eq!(
        kit::zone_of(&engine, board.opal),
        Zone::Graveyard,
        "and so is the legendary artifact"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all of {{5}}{{B}}{{R}} was spent"
    );
}

/// "Sacrifice Mount Doom and a legendary artifact": an artifact that is not
/// legendary does not pay for it. With seven mana floating and Mount Doom
/// untapped, the wipe is offered beside a Mox Opal (a legendary artifact) and is
/// not offered beside a Sol Ring (an artifact, but not a legendary one), while
/// the `{1}{B}{R}` ability is affordable in both.
#[test]
fn mount_doom_wipe_needs_a_legendary_artifact() {
    let p0 = kit::P0;
    let offered_beside = |artifact: CardIndex| {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    mount_doom(),
                    forest(),
                    forest(),
                    forest(),
                    forest(),
                    forest(),
                    swamp(),
                    mountain(),
                    artifact,
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let doom = on_battlefield(&engine, p0, mount_doom()).expect("Mount Doom");
        tap_mana_except(&mut engine, p0, doom);
        kit::offered(&engine, doom)
    };

    let with_the_opal = offered_beside(mox_opal());
    assert!(
        with_the_opal.contains(&WIPE),
        "a legendary artifact to sacrifice, so the wipe is offered: {with_the_opal:?}"
    );

    let with_the_ring = offered_beside(sol_ring());
    assert!(
        with_the_ring.contains(&PING),
        "the `{{1}}{{B}}{{R}}` ability is affordable and offered: {with_the_ring:?}"
    );
    assert!(
        !with_the_ring.contains(&WIPE),
        "no legendary artifact to sacrifice, so no wipe: {with_the_ring:?}"
    );
}

/// "Activate only as a sorcery."
///
/// The same seven mana floating, the same Mox Opal to sacrifice: in the first
/// main phase with an empty stack the wipe is offered; in the declare
/// attackers step, with the Ogre attacking, it is not (while the `{1}{B}{R}`
/// ability, which has no timing restriction, still is). Two games, because
/// mana does not outlast its step.
#[test]
fn mount_doom_wipe_is_only_offered_at_sorcery_speed() {
    let (p0, p1) = (kit::P0, kit::P1);

    let (mut engine, board) = the_board();
    tap_mana_except(&mut engine, p0, board.doom);
    let in_main = kit::offered(&engine, board.doom);
    assert!(
        in_main.contains(&WIPE),
        "in the main phase with an empty stack the wipe is offered: {in_main:?}"
    );

    let (mut engine, board) = the_board();
    kit::declare_attack(&mut engine, &[board.mine[0]], p1);
    tap_mana_except(&mut engine, p0, board.doom);
    let in_combat = kit::offered(&engine, board.doom);
    assert!(
        in_combat.contains(&PING),
        "the instant-speed ability is offered in combat: {in_combat:?}"
    );
    assert!(
        !in_combat.contains(&WIPE),
        "the wipe is a sorcery-speed ability and combat is not a main phase: {in_combat:?}"
    );
}

/// "…then destroy the rest": destroy, not sacrifice. A creature with
/// indestructible survives (CR 702.12b), and a creature with a regeneration
/// shield is regenerated instead of destroyed (CR 701.19): it is tapped and
/// still on the battlefield afterwards.
///
/// Player 0 also controls a Lotleth Troll (regenerates for `{B}`) that has
/// bought a shield, and player 1 a Darksteel Gargoyle (indestructible). Nobody
/// is chosen: the two Ogres die, the Troll and the Gargoyle live.
#[test]
fn mount_doom_destroys_so_indestructible_and_regenerating_creatures_survive() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mount_doom(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                swamp(),
                mountain(),
                mox_opal(),
                gray_ogre(),
                lotleth_troll(),
            ],
        )
        .battlefield(1, &[gray_ogre(), darksteel_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let doom = on_battlefield(&engine, p0, mount_doom()).expect("Mount Doom");
    let opal = on_battlefield(&engine, p0, mox_opal()).expect("the Opal");
    let my_ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll");
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let gargoyle = on_battlefield(&engine, p1, darksteel_gargoyle()).expect("the Gargoyle");

    tap_mana_except(&mut engine, p0, doom);
    raise_a_shield(&mut engine, p0, troll, 1);
    kit::activate_object(&mut engine, p0, doom, WIPE);
    kit::settle(&mut engine, p0, |_, pending| match pending {
        Pending::ChooseCards { options, .. } if options.contains(&opal) => {
            kit::choose_cards(pending, &[opal])
        }
        Pending::ChooseCards { .. } => kit::choose_cards(pending, &[]),
        _ => None,
    });

    assert_eq!(kit::zone_of(&engine, my_ogre), Zone::Graveyard, "destroyed");
    assert_eq!(
        kit::zone_of(&engine, their_ogre),
        Zone::Graveyard,
        "destroyed"
    );
    assert!(
        kit::is_on_battlefield(&engine, gargoyle),
        "indestructible: destroying it does nothing"
    );
    assert!(
        kit::is_on_battlefield(&engine, troll),
        "the regeneration shield replaced the destruction"
    );
    assert!(is_tapped(&engine, troll), "regenerating taps the creature");
    assert_eq!(
        engine
            .state()
            .object(troll)
            .expect("the Troll")
            .regeneration_shields,
        0,
        "and uses the shield up"
    );
}

/// "{1}{B}{R}, {T}: Mount Doom deals 1 damage to each opponent."
///
/// At a table of three both opponents are dealt 1 damage and Mount Doom's
/// controller none; the `{1}{B}{R}` is spent and Mount Doom is tapped.
#[test]
fn mount_doom_deals_one_damage_to_each_opponent() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[
                mount_doom(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let doom = on_battlefield(&engine, p0, mount_doom()).expect("mount doom on battlefield");

    // Float {5}{B}{R} from basics while keeping Mount Doom untapped.
    tap_mana_except(&mut engine, p0, doom);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    let offered = kit::offered(&engine, doom);
    assert!(offered.contains(&MANA), "ability 0 is offered: {offered:?}");
    assert!(offered.contains(&PING), "ability 1 is offered: {offered:?}");
    assert!(
        !offered.contains(&WIPE),
        "with no legendary artifact to sacrifice the wipe is not: {offered:?}"
    );

    activate(&mut engine, p0, mount_doom(), PING);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::life(&engine, p1), 19, "one opponent takes 1 damage");
    assert_eq!(kit::life(&engine, p2), 19, "and so does the other");
    assert_eq!(kit::life(&engine, p0), 20, "controller takes no damage");
    assert!(is_tapped(&engine, doom));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{1}}{{B}}{{R}} spent from floating pool"
    );
}

/// "{T}, Pay 1 life: Add {B} or {R}."
///
/// Once for each colour: the land taps for one mana of the colour chosen, its
/// controller pays 1 life, and Mount Doom is tapped.
#[test]
fn mount_doom_taps_for_black_or_red_and_costs_one_life() {
    let p0 = kit::P0;
    for colour in [ManaColor::Black, ManaColor::Red] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[mount_doom()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let doom = on_battlefield(&engine, p0, mount_doom()).expect("Mount Doom");

        activate(&mut engine, p0, mount_doom(), MANA);
        if let Pending::ChooseColor { options, .. } = engine.pending().clone() {
            assert!(
                options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
                "black or red: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(colour))
                .expect("the colour was offered");
        }

        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "one mana");
        assert_eq!(pool.available(colour), 1, "of the colour chosen");
        assert_eq!(kit::life(&engine, p0), 19, "the price is 1 life");
        assert!(is_tapped(&engine, doom), "and the land is tapped");
    }
}
