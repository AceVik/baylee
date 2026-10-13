//! `cards/lands/pain/contested_war_zone.rs`, played.
//!
//! Oracle: "Whenever a creature deals combat damage to you, that creature's
//! controller gains control of this land. {T}: Add {C}. {1}, {T}: Attacking
//! creatures get +1/+0 until end of turn."
//!
//! Written from that text, the four Scryfall rulings and the Comprehensive
//! Rules (CR 510.2, 603.2, 109.5, 611.2c). The ability numbers are not part of
//! the text, so the tests find the `{1}, {T}` pump and the `{T}` mana ability
//! by what they are (the one that is a mana ability and the one that is not),
//! never by position.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::AbilityDef;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

/// The number of the land's ability that is neither a mana ability nor a
/// trigger: the `{1}, {T}` pump.
#[track_caller]
fn pump_index(engine: &Engine<RegistryLookup>, zone: ObjectId) -> u32 {
    let object = engine.state().object(zone).expect("the land is out");
    let defs = object.abilities(&RegistryLookup);
    let found: Vec<u32> = defs
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(
                ability,
                AbilityDef::Activated { .. } | AbilityDef::ActivatedConditional { .. }
            ) && !ability.is_mana_ability()
        })
        .map(|(at, _)| u32::try_from(at).expect("a handful of abilities"))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one activated non-mana ability: {found:?}"
    );
    found[0]
}

/// The number of the land's `{T}: Add {C}`.
#[track_caller]
fn mana_index(engine: &Engine<RegistryLookup>, zone: ObjectId) -> u32 {
    let object = engine.state().object(zone).expect("the land is out");
    let defs = object.abilities(&RegistryLookup);
    let found: Vec<u32> = defs
        .iter()
        .enumerate()
        .filter(|(_, ability)| ability.is_mana_ability())
        .map(|(at, _)| u32::try_from(at).expect("a handful of abilities"))
        .collect();
    assert_eq!(found.len(), 1, "exactly one mana ability: {found:?}");
    found[0]
}

/// Who controls `object` now.
#[track_caller]
fn controller_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine
        .state()
        .object(object)
        .expect("the object exists")
        .controller
}

/// Seat 0 holds the land and a Wall of Wood, seat 1 a Grizzly Bears, at seat 0's
/// first main phase.
#[track_caller]
fn table() -> (Engine<RegistryLookup>, ObjectId) {
    let engine = lk::game(
        SEED,
        &[contested_war_zone(), index::WALL_OF_WOOD],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let zone = lk::obj(&engine, P0, contested_war_zone());
    (engine, zone)
}

/// Passes to seat 1's declare-attackers question (seat 0's own empty attack
/// on the way is declared by `pass_until`).
#[track_caller]
fn to_their_attack(engine: &mut Engine<RegistryLookup>) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
}

/// The active player declares `attackers` against `at`.
#[track_caller]
fn declare_attack(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId], at: PlayerId) {
    let Pending::ChooseAttackers { player, .. } = engine.pending().clone() else {
        panic!("no attacker question: {:?}", engine.pending());
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(at)))
                    .collect(),
            },
        )
        .expect("the attackers came out of the offer");
}

/// The attacker declares `attackers` against `at`; if `blocks` is not empty
/// the defender is asked and makes exactly those; then combat is played out to
/// the second main phase, triggers and all.
#[track_caller]
fn strike(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    at: PlayerId,
    blocks: &[(ObjectId, ObjectId)],
) {
    declare_attack(engine, attackers, at);
    if !blocks.is_empty() {
        pass_until(engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        lk::block(engine, blocks);
    }
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// "{1}, {T}: Attacking creatures get +1/+0 until end of turn." and "{T}: Add
/// {C}": the half of the card that is not a trigger.
///
/// The reading that costs a scenario is the word *attacking*: a 1/1 that
/// attacked is a 2/1, while an Elf under the same controller that stayed home
/// and an Elf across the table are both still the 1/1s they were printed as.
/// The `{1}` is a real payment -- the pool is filled before the ability is
/// claimed and one mana is gone afterwards -- and the land is kept standing
/// because its own `{T}` is half of what the ability charges.
#[test]
fn contested_war_zone_pumps_the_creatures_that_attacked_and_only_those() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[contested_war_zone()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A land arrives by the land drop and not by being seeded: seeded
    // permanents are placed rather than played, and everything this card does
    // hangs off the permanent that arrived.
    play_land(&mut engine, p0, contested_war_zone());
    let zone = on_battlefield(&engine, p0, contested_war_zone()).expect("the land is out");
    assert_eq!(
        engine.state().object(zone).map(|o| o.controller),
        Some(p0),
        "and it belongs to the seat that played it"
    );
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays home");
    let (attacker, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // "Attacking creature" is a state the combat step hands out, so the
    // scenario starts at the declaration that puts a creature in it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 with no summoning sickness may attack: {attackers:?}"
    );
    let defender = defenders
        .into_iter()
        .next()
        .expect("the one opponent is attackable");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, defender)],
            },
        )
        .expect("the Elf was on the offer of who may attack");

    // Mana into the pool before the claim, and the War Zone named as the one
    // source kept standing: its `{T}` is what the ability charges, so tapping
    // it for `{C}` would remove the very half under test. CR 500.5 leaves the
    // pool through this step.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, Some(contested_war_zone()));
    let pool = engine.state().players[0].mana_pool.total();
    assert!(pool >= 1, "the Forests and the Elves paid into the pool");

    // The pump, found by being the one activated ability that is not a mana
    // ability, not by its place in the list.
    let pump = pump_index(&engine, zone);
    activate(&mut engine, p0, contested_war_zone(), pump);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, attacker),
        (2, 1),
        "\"Attacking creatures get +1/+0\" — the creature that attacked"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf under the same controller that stayed home is not attacking"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and neither is the creature across the table that never attacked"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool - 1,
        "the {{1}} came out of the pool the lands and Elves filled"
    );
    assert!(
        on_battlefield(&engine, p0, contested_war_zone()).is_some(),
        "the land is still on the table: the pump is the ability and not its source"
    );
}

/// "Whenever a creature deals combat damage to you, that creature's controller
/// gains control of this land." Seat 0 holds the land and seat 1's Grizzly Bears
/// connects: the land is seat 1's afterwards. Ruling: "Gaining control of
/// Contested War Zone doesn't cause it to tap or untap" -- played with the
/// land standing and with it tapped, and it is as it was in both.
#[test]
fn the_controller_of_a_creature_that_hits_you_gains_the_land_without_tapping_or_untapping_it() {
    for tapped in [false, true] {
        let (mut engine, zone) = table();
        if tapped {
            engine.dev_state_mut(P0).unwrap().set_tapped(zone, true);
            engine.refresh_offer();
        }
        assert_eq!(lk::tapped(&engine, zone), tapped, "the scene is as set");
        assert_eq!(controller_of(&engine, zone), P0, "seat 0 holds the land");
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        let life = lk::life(&engine, P0);

        to_their_attack(&mut engine);
        strike(&mut engine, &[bears], P0, &[]);

        assert_eq!(
            lk::life(&engine, P0),
            life - 2,
            "the Bears connected: combat damage was dealt to seat 0"
        );
        assert_eq!(
            lk::tapped(&engine, zone),
            tapped,
            "the change of control neither tapped nor untapped it"
        );
        assert_eq!(
            controller_of(&engine, zone),
            P1,
            "the Bears' controller gained control of the land (tapped: {tapped})"
        );
    }
}

/// "… deals combat damage to *you*": a hit on another player is no concern of
/// the land's controller. At a three-seat table seat 1's Bears connect with
/// seat 2 and the land stays with seat 0; the control is the same Bears
/// connecting with seat 0, which takes it.
#[test]
fn damage_dealt_to_another_player_does_not_move_the_land() {
    for (victim, holder_afterwards) in [(P2, P0), (P0, P1)] {
        let mut engine = Duel::table(SEED, index::FOREST, 3)
            .battlefield(0, &[contested_war_zone()])
            .battlefield(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, P0), "seat 0 reaches its main");
        let zone = lk::obj(&engine, P0, contested_war_zone());
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        let life = lk::life(&engine, victim);

        to_their_attack(&mut engine);
        strike(&mut engine, &[bears], victim, &[]);

        assert_eq!(
            lk::life(&engine, victim),
            life - 2,
            "the Bears connected with seat {}",
            victim.get()
        );
        assert_eq!(
            controller_of(&engine, zone),
            holder_afterwards,
            "after a hit on seat {}",
            victim.get()
        );
    }
}

/// A blocked creature deals its combat damage to the blocker, not to you. The
/// Bears are blocked by a Wall of Wood and the land stays; unblocked, the same
/// Bears take it.
#[test]
fn a_creature_that_only_hits_a_blocker_does_not_take_the_land() {
    for (blocked, holder_afterwards, life_lost) in [(true, P0, 0), (false, P1, 2)] {
        let (mut engine, zone) = table();
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        let wood = lk::obj(&engine, P0, index::WALL_OF_WOOD);
        let life = lk::life(&engine, P0);

        to_their_attack(&mut engine);
        let blocks = if blocked { vec![(wood, bears)] } else { vec![] };
        strike(&mut engine, &[bears], P0, &blocks);

        assert_eq!(
            lk::life(&engine, P0),
            life - life_lost,
            "life after the combat (blocked: {blocked})"
        );
        if blocked {
            assert_eq!(
                lk::damage_on(&engine, wood),
                2,
                "the Bears' damage went to the Wall"
            );
        }
        assert_eq!(
            controller_of(&engine, zone),
            holder_afterwards,
            "the land after the combat (blocked: {blocked})"
        );
    }
}

/// "… combat damage": a creature that damages you any other way takes nothing.
/// Seat 1's Prodigal Sorcerer pings seat 0 and the land is still seat 0's;
/// the Bears then attack the same seat and, with the same life total and the
/// same land, are the control that takes it.
#[test]
fn noncombat_damage_from_a_creature_does_not_move_the_land() {
    let mut engine = lk::game(
        SEED,
        &[contested_war_zone()],
        &[index::PRODIGAL_SORCERER, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let zone = lk::obj(&engine, P0, contested_war_zone());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    let life = lk::life(&engine, P0);
    lk::use_ability(&mut engine, P1, index::PRODIGAL_SORCERER, 0, &[], &[P0]);
    assert_eq!(
        lk::life(&engine, P0),
        life - 1,
        "the Sorcerer's one damage reached seat 0"
    );
    assert_eq!(
        controller_of(&engine, zone),
        P0,
        "damage that is not combat damage gives nothing"
    );

    to_their_attack(&mut engine);
    strike(&mut engine, &[bears], P0, &[]);
    assert_eq!(
        controller_of(&engine, zone),
        P1,
        "control: the Bears' combat damage does"
    );
}

/// Two creatures of one controller deal combat damage to you together: two
/// triggers, and the land is that controller's afterwards (it changes hands
/// once, whatever the second trigger does).
#[test]
fn two_creatures_of_one_controller_hitting_you_still_leave_the_land_with_them() {
    let mut engine = lk::game(
        SEED,
        &[contested_war_zone()],
        &[index::GRIZZLY_BEARS, index::GRAY_OGRE],
        &[],
        &[],
    );
    let zone = lk::obj(&engine, P0, contested_war_zone());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let ogre = lk::obj(&engine, P1, index::GRAY_OGRE);
    let life = lk::life(&engine, P0);
    to_their_attack(&mut engine);
    strike(&mut engine, &[bears, ogre], P0, &[]);
    assert_eq!(lk::life(&engine, P0), life - 4, "both connected");
    assert_eq!(controller_of(&engine, zone), P1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == contested_war_zone()))
            })
            .count(),
        1,
        "still one land: control moved, nothing was copied or lost"
    );
}

/// Ruling: "The word 'you' in a permanent's trigger condition always refers to
/// the permanent's current controller." Seat 1's Bears take the land on
/// seat 1's turn; on seat 0's turn their Gray Ogre hits seat 1, the land's
/// controller now, and takes it back. If "you" stayed the first controller the
/// Ogre's hit would give it nothing.
#[test]
fn you_is_the_lands_current_controller_so_it_changes_hands_again() {
    let mut engine = lk::game(
        SEED,
        &[contested_war_zone(), index::GRAY_OGRE],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let zone = lk::obj(&engine, P0, contested_war_zone());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let ogre = lk::obj(&engine, P0, index::GRAY_OGRE);

    to_their_attack(&mut engine);
    strike(&mut engine, &[bears], P0, &[]);
    let after_their_turn = controller_of(&engine, zone);

    // The Bears are tapped from their attack, so nothing of seat 1's blocks.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let life = lk::life(&engine, P1);
    strike(&mut engine, &[ogre], P1, &[]);
    assert_eq!(lk::life(&engine, P1), life - 2, "the Ogre connected");
    assert_eq!(after_their_turn, P1, "seat 1 took it on its own turn");
    assert_eq!(
        controller_of(&engine, zone),
        P0,
        "damage to seat 1, the land's controller by then, gave it back to the Ogre's"
    );
}

/// Ruling: "The last activated ability affects all attacking creatures, not
/// just those you control." Seat 1's Bears attack seat 0, which pays `{1}` and
/// taps the land: the opposing attacker is a 3/2 and seat 0's own Hill Giant,
/// at home, is still a 3/3. "Until end of turn": at seat 0's next upkeep the
/// Bears are a 2/2 again.
#[test]
fn the_pump_reaches_attackers_of_every_controller_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[
            contested_war_zone(),
            index::FOREST,
            index::FOREST,
            index::HILL_GIANT,
        ],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let zone = lk::obj(&engine, P0, contested_war_zone());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    assert_eq!(pt(&engine, bears), (2, 2), "a printed 2/2");

    to_their_attack(&mut engine);
    declare_attack(&mut engine, &[bears], P0);
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana_but(&mut engine, P0, Some(contested_war_zone()));
    assert!(
        lk::pool_total(&engine, P0) >= 1,
        "the Forests paid the {{1}} into the pool"
    );
    let pump = pump_index(&engine, zone);
    activate(&mut engine, P0, contested_war_zone(), pump);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (3, 2),
        "the opponent's attacking creature gets +1/+0 too"
    );
    assert_eq!(
        pt(&engine, giant),
        (3, 3),
        "seat 0's own creature is not attacking and is untouched"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "\"until end of turn\": a turn later it is the printed 2/2"
    );
}

/// "{T}: Add {C}": one colourless mana into the pool, and the land is tapped
/// for it.
#[test]
fn the_land_taps_for_one_colourless_mana() {
    let (mut engine, zone) = table();
    assert_eq!(lk::pool_total(&engine, P0), 0, "the pool starts empty");
    assert!(!lk::tapped(&engine, zone), "the land starts untapped");
    let mana = mana_index(&engine, zone);
    activate(&mut engine, P0, contested_war_zone(), mana);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Colorless),
        1,
        "one {{C}}"
    );
    assert_eq!(lk::pool_total(&engine, P0), 1, "and nothing else");
    assert!(lk::tapped(&engine, zone), "the cost was the land's own tap");
}
