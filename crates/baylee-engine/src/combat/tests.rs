//! Combat's tests: declarations, restrictions, damage assignment and banding.

mod assignment_restrictions;
mod battle;
mod exchange;
mod history;
mod ice_landwalk;
use super::block::LANDWALKS;
use super::damage::deal_combat_damage;
use super::*;
use crate::event::DamageTarget;
use crate::object::ObjectKind;
use crate::state::CardLookup;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::KeywordSet;
use baylee_core::color::Color;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{CardIndex, SubtypeId};
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};

/// A registry with nothing in it: these tests build creatures directly
/// rather than going through cards, because the interactions under
/// test are between *keywords*, and picking real cards that happen to
/// carry them would make the test about those cards.
struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}

fn empty_state() -> GameState {
    seated(2)
}

/// An empty board with `seats` players at it.
fn seated(seats: usize) -> GameState {
    let seat = || SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![],
        sideboard: vec![],
        commanders: vec![],
        starting_life: Some(20),
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GameState::from_preset(
        &GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 1,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: (0..seats).map(|_| seat()).collect(),
        },
        &NoCards,
    )
    .expect("an empty board")
}

fn creature(
    state: &mut GameState,
    controller: PlayerId,
    power: i16,
    toughness: i16,
    keywords: KeywordSet,
) -> ObjectId {
    let name = state.names.intern("Test Creature");
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let b = state.object_mut(id).expect("just created").base_mut();
    b.types = TypeSet::CREATURE;
    b.power = Some(power);
    b.toughness = Some(toughness);
    b.keywords = keywords;
    id
}

fn attack(state: &mut GameState, creature: ObjectId, defending: PlayerId) {
    state.combat.declare_attackers([AttackerInfo {
        creature,
        defending: Defender::Player(defending),
        blocked: false,
        band: None,
    }]);
}

/// Declares `creature` as attacking a planeswalker instead of a seat.
fn attack_walker(state: &mut GameState, creature: ObjectId, walker: ObjectId) {
    state.combat.declare_attackers([AttackerInfo {
        creature,
        defending: Defender::Planeswalker(walker),
        blocked: false,
        band: None,
    }]);
}

/// A planeswalker on the battlefield with `loyalty` counters.
fn planeswalker(state: &mut GameState, controller: PlayerId, loyalty: u16) -> ObjectId {
    let name = state.names.intern("Test Walker");
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let obj = state.object_mut(id).expect("just created");
    let b = obj.base_mut();
    b.types = TypeSet::PLANESWALKER;
    b.loyalty = Some(loyalty);
    obj.counters
        .set(baylee_cards_dsl::CounterKind::Loyalty, loyalty);
    id
}

fn loyalty(state: &GameState, id: ObjectId) -> u16 {
    state.object(id).map_or(0, |o| {
        o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
    })
}

fn block(state: &mut GameState, blocker: ObjectId, attacker: ObjectId) {
    state.declare_block(blocker, attacker);
}

fn damage(state: &GameState, id: ObjectId) -> u16 {
    state.object(id).map_or(0, |o| o.damage)
}

fn on_battlefield(state: &GameState, id: ObjectId) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A blink takes a creature out of combat and brings back a different
/// one (CR 506.4, CR 400.7) — different to the rules, at any rate; the
/// `ObjectId` is an arena handle and comes back unchanged, which is why
/// nothing noticed. Ephemerate on an attacking Solemn Simulacrum left it
/// declared as an attacker, and a Restoration Angel later took an
/// attacking Sun Titan and Elesh Norn out of a combat they went on
/// fighting.
/// A board of `teams.len()` seats, on the sides it names.
fn teamed_state(teams: &[Option<u8>]) -> GameState {
    let seat = |team: Option<u8>| SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![],
        sideboard: vec![],
        commanders: vec![],
        starting_life: Some(20),
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team,
    };
    GameState::from_preset(
        &GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 1,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: teams.iter().copied().map(seat).collect(),
        },
        &NoCards,
    )
    .expect("a seated board")
}

/// What may be attacked is **each surviving opponent and the
/// planeswalkers those opponents control** (CR 506.2), as one flat list
/// because the rules make it one choice and a client renders it as one.
///
/// A teammate is not an opponent, and neither is their planeswalker —
/// the walker half reads the same opponent list, which is the only
/// reason it cannot answer differently. My own walker is not a defender
/// either, and a seat that has lost is no longer anybody's opponent.
#[test]
fn a_defender_is_an_opponent_or_something_an_opponent_controls() {
    let mut state = teamed_state(&[Some(1), Some(2), Some(1), Some(2)]);
    let me = PlayerId::new(0);
    let ally = PlayerId::new(2);
    let enemy = PlayerId::new(1);
    let other_enemy = PlayerId::new(3);

    let mine = planeswalker(&mut state, me, 4);
    let allys = planeswalker(&mut state, ally, 4);
    let theirs = planeswalker(&mut state, enemy, 4);

    let options = defender_options(&state, me);
    assert_eq!(
        options,
        vec![
            Defender::Player(enemy),
            Defender::Player(other_enemy),
            Defender::Planeswalker(theirs),
        ],
        "the two seats across the table and the one walker they control"
    );
    assert!(!options.contains(&Defender::Player(ally)));
    assert!(
        !options.contains(&Defender::Planeswalker(allys)),
        "a teammate's planeswalker is not a defender"
    );
    assert!(!options.contains(&Defender::Planeswalker(mine)));

    // A seat that has lost is nobody's opponent any more, and its
    // planeswalker goes with it.
    state
        .players
        .iter_mut()
        .find(|p| p.id == enemy)
        .expect("seated")
        .loss = Some(crate::event::LossReason::Conceded);
    assert_eq!(
        defender_options(&state, me),
        vec![Defender::Player(other_enemy)],
        "and the walker it controlled is no longer reachable either"
    );
}

/// In a duel every other seat is an opponent, which is the case that
/// makes the team reading above invisible: with no teams on the table
/// the two answers are the same list.
#[test]
fn with_no_teams_on_the_table_every_other_seat_is_a_defender() {
    let mut state = empty_state();
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let walker = planeswalker(&mut state, them, 3);
    creature(&mut state, them, 2, 2, KeywordSet::EMPTY);

    assert_eq!(
        defender_options(&state, me),
        vec![Defender::Player(them), Defender::Planeswalker(walker)],
        "a creature they control is not something to attack"
    );
    assert_eq!(defender_options(&state, them), vec![Defender::Player(me)]);
}

/// The damage aimed at a defender goes to a seat, and which seat is a
/// second question: a planeswalker's controller rather than the player
/// it was declared against.
///
/// `None` once the walker has left. The attack stays declared
/// (CR 506.4c) — this is what says there is nothing left to damage,
/// which is the difference between a trampling attacker having a
/// recipient and having none.
#[test]
fn the_damage_goes_to_a_seat_and_a_walker_that_left_names_none() {
    let mut state = empty_state();
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let walker = planeswalker(&mut state, them, 3);

    assert_eq!(defending_player(&state, Defender::Player(them)), Some(them));
    assert_eq!(
        defending_player(&state, Defender::Player(me)),
        Some(me),
        "a seat names itself whoever is asking"
    );
    assert_eq!(
        defending_player(&state, Defender::Planeswalker(walker)),
        Some(them),
        "the walker's controller, not whoever was attacked"
    );

    state
        .move_object(
            walker,
            ZoneLocation::Graveyard(them),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::StateBased,
        )
        .expect("it dies");
    assert_eq!(
        defending_player(&state, Defender::Planeswalker(walker)),
        None,
        "the attack is still declared and there is nothing to damage"
    );
    assert_eq!(
        defending_player(&state, Defender::Planeswalker(ObjectId::new(9_999, 0))),
        None,
        "and an id that never was anything answers the same way"
    );
}

#[test]
fn a_blinked_attacker_is_out_of_combat() {
    let mut state = empty_state();
    let titan = creature(&mut state, P0, 6, 6, KeywordSet::EMPTY);
    let wall = creature(&mut state, P1, 0, 8, KeywordSet::EMPTY);
    attack(&mut state, titan, P1);
    block(&mut state, wall, titan);

    // Out and straight back in, which is what a blink is.
    state
        .move_object(
            titan,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("exiled");
    state
        .move_object(
            titan,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("returned");
    assert!(on_battlefield(&state, titan), "the blink brought it back");
    assert!(
        state.combat.attackers().is_empty(),
        "it is not attacking any more"
    );
    assert!(
        state.combat.blockers.is_empty(),
        "and the wall has nothing left to block"
    );

    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, wall), 0, "it dealt no damage");
    assert_eq!(damage(&state, titan), 0, "and took none");
    assert_eq!(state.players[1].life, 20, "nor did anything get through");
}

/// "Had become blocked by only that creature this combat" reads every
/// block made, not the blocks left: a second blocker that has since left
/// combat still counts, an attacker nothing blocked was blocked by
/// nobody, and `unblock` clears the flag only of the attacker it names.
#[test]
fn blocked_only_by_reads_the_combats_blocks_and_not_the_blocks_left() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
    let other = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let first = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    let second = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    state
        .combat
        .declare_attackers([attacker, other].map(|creature| AttackerInfo {
            creature,
            defending: Defender::Player(P1),
            blocked: false,
            band: None,
        }));
    state.declare_block(first, attacker);
    assert!(state.blocked_only_by(attacker, first));
    assert!(!state.blocked_only_by(other, first), "never blocked");
    state.declare_block(second, attacker);
    state.combat.remove_from_combat(second);
    assert!(
        !state.blocked_only_by(attacker, first),
        "the second blocker left, and still blocked it this combat"
    );
    state.declare_block(first, other);
    state.combat.unblock(other);
    assert!(!state.combat.is_blocked(other));
    assert!(state.combat.is_blocked(attacker));
}

/// `is_attacking` answers from a sorted index kept beside the attacker
/// list, so each door that changes the list must move the index with it:
/// a declaration out of id order, a creature leaving combat, and the
/// filter a player leaving the game runs (CR 800.4a).
#[test]
fn the_attacking_index_follows_every_change_to_the_attackers() {
    let mut state = empty_state();
    let ids: Vec<ObjectId> = (0..4)
        .map(|_| creature(&mut state, P0, 1, 1, KeywordSet::EMPTY))
        .collect();
    let idle = creature(&mut state, P0, 1, 1, KeywordSet::EMPTY);
    let agree = |state: &GameState| {
        for id in ids.iter().chain([&idle]) {
            assert_eq!(
                state.combat.is_attacking(*id),
                state.combat.attackers().iter().any(|a| a.creature == *id),
                "the index and the list disagree about {id:?}"
            );
        }
    };
    for id in ids.iter().rev() {
        attack(&mut state, *id, P1);
    }
    agree(&state);
    assert!(!state.combat.is_attacking(idle), "it was never declared");
    assert_eq!(
        state.combat.attackers()[0].creature,
        ids[3],
        "the list keeps declaration order; only the index is sorted"
    );

    state.combat.remove_from_combat(ids[1]);
    agree(&state);
    assert!(!state.combat.is_attacking(ids[1]));

    state.combat.retain_attackers(|a| a.creature != ids[2]);
    agree(&state);
    assert!(!state.combat.is_attacking(ids[2]));
    assert!(state.combat.is_attacking(ids[0]) && state.combat.is_attacking(ids[3]));
}

/// The other half, and the asymmetry that makes it its own case
/// (CR 509.1h): an attacker whose blocker leaves stays **blocked**. It
/// deals its damage to nothing at all — the blocker is gone and the
/// player is not a legal recipient.
#[test]
fn an_attacker_stays_blocked_when_its_blocker_is_blinked() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 7, 7, KeywordSet::EMPTY);
    let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    block(&mut state, chump, bear);

    state
        .move_object(
            chump,
            ZoneLocation::Exile(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("exiled");
    state
        .move_object(
            chump,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("returned");
    assert!(
        state.combat.is_blocked(bear),
        "being blocked is a fact about the attacker, not about the blocker"
    );
    assert!(state.combat.blockers_of(bear).is_empty());

    deal_combat_damage(&mut state, false);
    assert_eq!(state.players[1].life, 20, "seven damage went nowhere");
    assert_eq!(
        damage(&state, chump),
        0,
        "the creature that came back was never in this combat"
    );
}

/// "Blocking creature" and "unblocked creature" (CR 509.1g, 509.1h): an
/// attacker is neither blocked nor unblocked before blockers are
/// declared, and one that was blocked stays blocked with its blocker
/// gone, while the creature that came back is blocking nothing.
#[test]
fn blocking_and_unblocked_read_the_declarations() {
    use baylee_cards_dsl::Filter;
    let is = |state: &GameState, filter: &Filter, id: ObjectId| {
        crate::eval::matches(filter, state, state.object(id).expect("here"), P1, id)
    };
    let mut state = empty_state();
    let blocked = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let free = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let wall = creature(&mut state, P1, 0, 4, KeywordSet::EMPTY);
    attack(&mut state, blocked, P1);
    attack(&mut state, free, P1);
    state.turn.step = crate::turn::Step::DeclareAttackers;
    assert!(
        !is(&state, &Filter::Unblocked, free),
        "not before blockers are declared"
    );

    state.turn.step = crate::turn::Step::DeclareBlockers;
    block(&mut state, wall, blocked);
    assert!(is(&state, &Filter::Unblocked, free));
    assert!(!is(&state, &Filter::Unblocked, blocked));
    assert!(
        !is(&state, &Filter::Unblocked, wall),
        "a blocker attacks nothing"
    );
    assert!(is(&state, &Filter::Blocking, wall));
    assert!(!is(&state, &Filter::Blocking, free));

    for to in [ZoneLocation::Exile(P1), ZoneLocation::Battlefield] {
        state
            .move_object(
                wall,
                to,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("blinked");
    }
    assert!(
        !is(&state, &Filter::Blocking, wall),
        "the creature that came back blocks nothing"
    );
    assert!(
        !is(&state, &Filter::Unblocked, blocked),
        "and what it blocked stays blocked"
    );
}

/// …unless it tramples, which is the exception the owner asked for by
/// name: CR 702.19b assigns everything past the (absent) blockers to
/// what the creature was attacking.
#[test]
fn trample_goes_through_when_the_blocker_is_blinked() {
    let mut state = empty_state();
    let beast = creature(&mut state, P0, 7, 7, KeywordSet::TRAMPLE);
    let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    attack(&mut state, beast, P1);
    block(&mut state, chump, beast);

    state
        .move_object(
            chump,
            ZoneLocation::Exile(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("exiled");
    state
        .move_object(
            chump,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("returned");

    deal_combat_damage(&mut state, false);
    assert_eq!(
        state.players[1].life, 13,
        "all seven trample through, nothing having to be assigned first"
    );
}

/// Who divides a creature's damage when it blocks a band: the active
/// player while a creature it blocks has banding (CR 702.22k), and the
/// blocker's own controller once none has (CR 510.1d) — the band itself
/// outlives the keyword (CR 702.22e), so the question is asked of the
/// creatures as they are now.
#[test]
fn a_blocker_on_a_band_that_lost_banding_is_divided_by_its_controller() {
    let mut state = empty_state();
    let first = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let second = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
    attack(&mut state, first, P1);
    attack(&mut state, second, P1);
    state.combat.form_band(&[first, second]);
    block(&mut state, blocker, first);
    block(&mut state, blocker, second);

    let owed = divisions_owed(&state, false);
    assert_eq!(
        owed,
        vec![OwedDivision {
            source: blocker,
            chooser: P1,
            recipients: vec![first, second],
            amount: 3,
        }],
        "no creature it blocks has banding: its controller divides"
    );

    state.object_mut(first).expect("there").base_mut().keywords = KeywordSet::BANDING;
    state.refresh_characteristics();
    let owed = divisions_owed(&state, false);
    assert_eq!(owed.len(), 1);
    assert_eq!(
        owed[0].chooser, state.turn.active,
        "banding hands it to the active player"
    );
    assert_ne!(
        state.turn.active, P1,
        "and that is not the blocker's controller"
    );
}

/// A creature blocking two deals its combat damage once, as divided
/// (CR 510.1d): 1 and 2 of its 3, not 3 to each.
#[test]
fn a_blocker_on_two_creatures_deals_its_power_once() {
    let mut state = empty_state();
    let first = creature(&mut state, P0, 0, 4, KeywordSet::EMPTY);
    let second = creature(&mut state, P0, 0, 4, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
    attack(&mut state, first, P1);
    attack(&mut state, second, P1);
    state.combat.form_band(&[first, second]);
    block(&mut state, blocker, first);
    block(&mut state, blocker, second);
    state.combat.record_division(Division {
        source: blocker,
        shares: vec![(first, 1), (second, 2)],
    });

    deal_combat_damage(&mut state, false);
    assert_eq!((damage(&state, first), damage(&state, second)), (1, 2));
    assert!(
        state.combat.division(blocker).is_none(),
        "a division is spent by the step it was chosen for"
    );
}

/// CR 702.2b + 704.5h: any nonzero damage from a deathtouch source is
/// lethal. A 1/1 deathtoucher marks one damage on a 6/6 and the SBA
/// pass has to destroy it, even though one is nowhere near six.
#[test]
fn a_point_of_deathtouch_damage_is_lethal() {
    let mut state = empty_state();
    let biter = creature(&mut state, P0, 1, 1, KeywordSet::DEATHTOUCH);
    let bear = creature(&mut state, P1, 6, 6, KeywordSet::EMPTY);
    attack(&mut state, biter, P1);
    block(&mut state, bear, biter);

    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, bear), 1, "one damage, not six");
    assert!(
        state.object(bear).expect("still alive").deathtouched,
        "the deathtouch mark is what the SBA reads"
    );

    crate::sba::run(&mut state, &NoCards);
    // These test creatures are card-less, so dying takes them out of
    // the game entirely (CR 704.5e) rather than to a graveyard.
    assert!(
        !on_battlefield(&state, bear),
        "the 6/6 dies to one point of deathtouch damage"
    );
}

/// The deathtouch window is "since the last SBA check" (CR 704.5h), so
/// an indestructible creature that survived the mark must not die when
/// the next unrelated SBA pass runs.
#[test]
fn deathtouch_does_not_linger_past_the_sba_that_judged_it() {
    let mut state = empty_state();
    let biter = creature(&mut state, P0, 1, 1, KeywordSet::DEATHTOUCH);
    let wall = creature(&mut state, P1, 0, 4, KeywordSet::INDESTRUCTIBLE);
    attack(&mut state, biter, P1);
    block(&mut state, wall, biter);
    deal_combat_damage(&mut state, false);

    crate::sba::run(&mut state, &NoCards);
    assert!(
        on_battlefield(&state, wall),
        "indestructible survives deathtouch (CR 702.12b)"
    );
    assert!(!state.object(wall).expect("alive").deathtouched);

    // Losing indestructibility later must not make it die retroactively.
    state.object_mut(wall).expect("alive").base_mut().keywords = KeywordSet::EMPTY;
    crate::sba::run(&mut state, &NoCards);
    assert!(
        on_battlefield(&state, wall),
        "the mark expired with the SBA pass that judged it"
    );
}

/// CR 510.4: first strike is a property of the creature dealing the
/// damage. A first-striking attacker must not drag its ordinary
/// blocker into the first-strike step — that is the whole point of
/// the keyword.
#[test]
fn first_strike_is_per_creature_not_per_combat() {
    let mut state = empty_state();
    let knight = creature(&mut state, P0, 3, 3, KeywordSet::FIRST_STRIKE);
    let bear = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, knight, P1);
    block(&mut state, bear, knight);

    deal_combat_damage(&mut state, true);
    assert_eq!(damage(&state, bear), 3, "the first striker connects");
    assert_eq!(
        damage(&state, knight),
        0,
        "the ordinary blocker does not strike first"
    );

    // The bear is dead before the regular step, so it never strikes.
    crate::sba::run(&mut state, &NoCards);
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, knight), 0, "the knight takes nothing back");
}

/// CR 510.1d: every blocking creature assigns its combat damage,
/// whether or not the attacker had damage left to assign to it. The
/// attacker's assignment loop must not gate the blockers' strikes.
#[test]
fn every_blocker_strikes_even_when_the_attacker_ran_out() {
    let mut state = empty_state();
    let small = creature(&mut state, P0, 1, 10, KeywordSet::EMPTY);
    let first = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let second = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
    attack(&mut state, small, P1);
    block(&mut state, first, small);
    block(&mut state, second, small);

    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, small), 5, "both blockers deal damage: 2 + 3");
}

/// A blocker with lifelink gains its controller life (CR 702.15b is
/// about the damage, not about who is attacking).
#[test]
fn a_blocker_with_lifelink_gains_life() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 1, 5, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 4, 4, KeywordSet::LIFELINK);
    attack(&mut state, attacker, P1);
    block(&mut state, blocker, attacker);

    deal_combat_damage(&mut state, false);
    assert_eq!(state.players[1].life, 24, "20 + the blocker's 4 power");
}

/// Trample with deathtouch only has to assign one damage per blocker
/// before the rest tramples over (CR 702.19b + 702.2b).
#[test]
fn trample_over_deathtouch_only_owes_one_per_blocker() {
    let mut state = empty_state();
    let beast = creature(
        &mut state,
        P0,
        5,
        5,
        KeywordSet::TRAMPLE.union(KeywordSet::DEATHTOUCH),
    );
    let wall = creature(&mut state, P1, 0, 4, KeywordSet::EMPTY);
    attack(&mut state, beast, P1);
    block(&mut state, wall, beast);

    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, wall), 1, "one point is lethal here");
    assert_eq!(state.players[1].life, 16, "the other four trample through");
}

/// Double strike (CR 702.4b): damage in *both* steps, and the same
/// creature deals its full power each time.
#[test]
fn a_double_striker_deals_damage_in_both_steps() {
    let mut state = empty_state();
    let hero = creature(&mut state, P0, 2, 2, KeywordSet::DOUBLE_STRIKE);
    let wall = creature(&mut state, P1, 0, 9, KeywordSet::EMPTY);
    attack(&mut state, hero, P1);
    block(&mut state, wall, hero);

    deal_combat_damage(&mut state, true);
    assert_eq!(
        damage(&state, wall),
        2,
        "no damage in the first-strike step"
    );
    deal_combat_damage(&mut state, false);
    assert_eq!(
        damage(&state, wall),
        4,
        "no second helping in the regular step"
    );
}

#[test]
fn assignment_reveal_cannot_retroactively_grant_an_extra_strike() {
    use crate::object::{Rider, Status};
    let mut state = empty_state();
    let first = creature(&mut state, P0, 1, 1, KeywordSet::FIRST_STRIKE);
    let masked = creature(&mut state, P0, 5, 5, KeywordSet::DOUBLE_STRIKE);
    let object = state.object_mut(masked).unwrap();
    object.original_base = Some(std::sync::Arc::clone(&object.base));
    let base = object.base_mut();
    base.power = Some(2);
    base.toughness = Some(2);
    base.keywords = KeywordSet::EMPTY;
    object.status.insert(Status::FACE_DOWN);
    object.riders.push(Rider::Masked);
    attack(&mut state, first, P1);
    attack(&mut state, masked, P1);
    deal_combat_damage(&mut state, true);
    assert_eq!(state.players[1].life, 19);
    assert!(
        state
            .object(masked)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    deal_combat_damage(&mut state, false);
    assert_eq!(
        state.players[1].life, 14,
        "revealed power assigns once in the already-required regular step"
    );
    assert!(
        !state
            .object(masked)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
}

/// The control: a plain first striker must *not* strike twice, which
/// is the only thing that makes the test above about double strike
/// rather than about the step machinery.
#[test]
fn a_first_striker_deals_damage_only_once() {
    let mut state = empty_state();
    let knight = creature(&mut state, P0, 2, 2, KeywordSet::FIRST_STRIKE);
    let wall = creature(&mut state, P1, 0, 9, KeywordSet::EMPTY);
    attack(&mut state, knight, P1);
    block(&mut state, wall, knight);

    deal_combat_damage(&mut state, true);
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, wall), 2, "first strike struck twice");
}

/// Defender (CR 702.3b): untapped, awake, and still not attacking.
#[test]
fn a_creature_with_defender_cannot_attack() {
    let mut state = empty_state();
    let wall = creature(&mut state, P0, 0, 4, KeywordSet::DEFENDER);
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    // Neither is summoning-sick: both were created before this turn.
    for player in &mut state.players {
        player.turn_start_timestamp = u64::MAX;
    }
    assert!(!can_attack(&state, P0, wall), "a wall attacked");
    assert!(can_attack(&state, P0, bear), "the control could not attack");
}

/// An "as though" permission on one creature, as an Aura's static line
/// lands on the effect table.
fn permit(state: &mut GameState, creature: ObjectId, modifier: baylee_cards_dsl::Modifier) {
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(state, creature);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Text,
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier,
    });
}

/// "Can attack as though it didn't have defender" (Animate Wall): the
/// wall attacks. An "as though" applies only to what it states (CR
/// 609.4), so a "can't attack" beside the defender still holds.
#[test]
fn a_wall_that_attacks_as_though_it_had_no_defender_attacks() {
    let mut state = empty_state();
    let wall = creature(&mut state, P0, 0, 4, KeywordSet::DEFENDER);
    let barred = creature(
        &mut state,
        P0,
        0,
        4,
        KeywordSet::DEFENDER.union(KeywordSet::CANT_ATTACK),
    );
    for player in &mut state.players {
        player.turn_start_timestamp = u64::MAX;
    }
    permit(
        &mut state,
        wall,
        baylee_cards_dsl::Modifier::AttacksDespiteDefender,
    );
    permit(
        &mut state,
        barred,
        baylee_cards_dsl::Modifier::AttacksDespiteDefender,
    );
    assert!(can_attack(&state, P0, wall), "the wall may attack");
    assert!(
        !can_attack(&state, P0, barred),
        "a \"can't attack\" is not defender, and still holds"
    );
}

/// "Can attack as though it had haste" (Instill Energy): the creature
/// attacks the turn it arrived, and is still summoning-sick for its {T}
/// abilities (CR 702.10c is not what the effect states).
#[test]
fn a_creature_that_attacks_as_though_it_had_haste_is_still_sick_for_its_tap() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let other = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    permit(
        &mut state,
        bear,
        baylee_cards_dsl::Modifier::AttacksAsThoughHaste,
    );
    assert!(
        can_attack(&state, P0, bear),
        "it attacks as though it had haste"
    );
    assert!(
        !can_attack(&state, P0, other),
        "the creature beside it is still asleep"
    );
    let obj = state.object(bear).expect("on the battlefield");
    assert!(summoning_sick(&state, obj), "and its tap still waits");
}

/// CR 302.6 is a rule about creatures, in both of its sentences. The
/// answer for anything else is no, and it is no *here* rather than at
/// each call site — a caller that forgot the type test used to get
/// "did this permanent enter this turn", which is a different question
/// with the same shape.
#[test]
fn nothing_but_a_creature_is_ever_summoning_sick() {
    let mut state = empty_state();
    let name = state.names.intern("Test Land");
    let land = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state
        .object_mut(land)
        .expect("just created")
        .base_mut()
        .types = TypeSet::LAND;
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);

    let land_obj = state.object(land).expect("on the battlefield");
    assert!(
        !summoning_sick(&state, land_obj),
        "a land played this turn was called asleep"
    );
    let bear_obj = state.object(bear).expect("on the battlefield");
    assert!(
        summoning_sick(&state, bear_obj),
        "a creature that entered this turn is asleep"
    );
}

/// The stamp a turn records is the *last one issued before it began*,
/// so the comparison against it is strict. Off by one the other way,
/// the last permanent to enter before a turn started woke up a turn
/// late — which nothing noticed, because the draw step almost always
/// issues a stamp in between.
#[test]
fn a_creature_that_was_already_there_when_the_turn_began_is_awake() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    // Exactly the boundary: the bear is the last thing stamped before
    // the turn began.
    let began = state.timestamp;
    for player in &mut state.players {
        player.turn_start_timestamp = began;
    }
    let obj = state.object(bear).expect("on the battlefield");
    assert!(!summoning_sick(&state, obj));
}

/// "Continuously since *their* most recent turn began" (CR 302.6). A
/// creature cast on your turn is still summoning sick through every
/// opponent's turn that follows: one shared turn clock woke it as soon
/// as anybody untapped, which handed its `{T}` to its controller a
/// whole turn early. Combat never saw the difference, because you only
/// declare attackers on your own turn.
#[test]
fn an_opponents_turn_does_not_wake_your_creature() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    // P1's turn has begun since the creature entered; P0's has not.
    state.players[1].turn_start_timestamp = state.timestamp;
    let obj = state.object(mine).expect("on the battlefield");
    assert!(
        summoning_sick(&state, obj),
        "an opponent untapping woke my creature"
    );

    // P0's own next turn is what wakes it.
    state.players[0].turn_start_timestamp = state.timestamp;
    let obj = state.object(mine).expect("on the battlefield");
    assert!(!summoning_sick(&state, obj));
}

/// Combat damage to a planeswalker takes loyalty off it (CR 306.8),
/// and leaves its controller's life alone.
#[test]
fn an_attack_on_a_planeswalker_costs_it_loyalty() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let walker = planeswalker(&mut state, P1, 5);
    attack_walker(&mut state, bear, walker);

    deal_combat_damage(&mut state, false);
    assert_eq!(loyalty(&state, walker), 3, "loyalty did not come off");
    assert_eq!(state.players[1].life, 20, "the player took the damage too");
    assert_eq!(damage(&state, walker), 0, "damage was marked on a walker");
}

/// Trample goes to whatever the creature is attacking (CR 702.19b) —
/// a planeswalker here, not past it to the player.
#[test]
fn trample_over_a_blocker_hits_the_planeswalker_being_attacked() {
    let mut state = empty_state();
    let beast = creature(&mut state, P0, 5, 5, KeywordSet::TRAMPLE);
    let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    let walker = planeswalker(&mut state, P1, 6);
    attack_walker(&mut state, beast, walker);
    block(&mut state, chump, beast);

    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, chump), 1, "the blocker takes lethal");
    assert_eq!(loyalty(&state, walker), 2, "the rest trampled elsewhere");
    assert_eq!(state.players[1].life, 20, "trample skipped the walker");
}

/// CR 506.4c: the attack survives the planeswalker leaving, but the
/// damage has nowhere to land — least of all on its controller.
#[test]
fn a_planeswalker_that_left_absorbs_nothing() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::LIFELINK);
    let walker = planeswalker(&mut state, P1, 5);
    attack_walker(&mut state, bear, walker);
    state
        .move_object(
            walker,
            ZoneLocation::Graveyard(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the walker leaves");

    let life_before = state.players[0].life;
    deal_combat_damage(&mut state, false);
    assert_eq!(state.players[1].life, 20, "the damage found the player");
    assert_eq!(
        state.players[0].life, life_before,
        "lifelink paid out for damage that was never dealt"
    );
}

#[test]
fn phased_out_creatures_cannot_attack_or_block() {
    let mut state = empty_state();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let attacker = creature(&mut state, p0, 2, 2, KeywordSet::HASTE);
    let blocker = creature(&mut state, p1, 2, 2, KeywordSet::default());
    assert!(can_attack(&state, p0, attacker));
    assert!(can_block(&state, p1, blocker, attacker));
    state
        .object_mut(blocker)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    assert!(!can_block(&state, p1, blocker, attacker));
    state
        .object_mut(blocker)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    state
        .object_mut(attacker)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    assert!(!can_attack(&state, p0, attacker));
    assert!(!can_block(&state, p1, blocker, attacker));
}

/// A land of `controller`'s with the one land type `subtype`.
fn land_of_type(state: &mut GameState, controller: PlayerId, subtype: SubtypeId) -> ObjectId {
    let name = state.names.intern("Test Land");
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let b = state.object_mut(id).expect("just created").base_mut();
    b.types = TypeSet::LAND;
    b.subtypes = baylee_core::types::SubtypeSet::from_slice(&[subtype]);
    id
}

/// Fog (a `ShieldKind::AllCombat` shield) prevents the combat damage
/// both writers here deal, to a player and to a creature; the old
/// writers dealt all of it.
#[test]
fn combat_damage_meets_the_prevention_shields() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
    let blocked = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    state.combat.declare_attackers([
        AttackerInfo {
            creature: attacker,
            defending: Defender::Player(P1),
            blocked: false,
            band: None,
        },
        AttackerInfo {
            creature: blocked,
            defending: Defender::Player(P1),
            blocked: false,
            band: None,
        },
    ]);
    state.declare_block(blocker, blocked);
    state.shields.push(crate::prevention::Shield {
        protects: crate::prevention::Shielded::Everything,
        kind: crate::prevention::ShieldKind::AllCombat,
        controller: P1,
    });
    deal_combat_damage(&mut state, false);
    assert_eq!(state.players[1].life, 20, "nothing got through");
    assert_eq!(state.object(blocker).unwrap().damage, 0);
    assert_eq!(state.object(blocked).unwrap().damage, 0);
    assert_eq!(state.shields.len(), 1, "Fog is never used up");
}

/// Fear (CR 702.36b): an artifact creature or a black one may block,
/// and nothing else — a green creature may not, a black artifact may.
#[test]
fn fear_is_blocked_only_by_artifact_or_black_creatures() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 2, 2, KeywordSet::FEAR);
    let plain = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let artifact = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    state.object_mut(artifact).unwrap().base_mut().types =
        TypeSet::CREATURE.union(TypeSet::ARTIFACT);
    let black = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    state.object_mut(black).unwrap().base_mut().colors =
        baylee_core::color::ColorSet::of(Color::Black);
    let green = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    state.object_mut(green).unwrap().base_mut().colors =
        baylee_core::color::ColorSet::of(Color::Green);

    assert!(
        !can_block(&state, P1, plain, attacker),
        "colourless, not an artifact"
    );
    assert!(!can_block(&state, P1, green, attacker));
    assert!(can_block(&state, P1, artifact, attacker));
    assert!(can_block(&state, P1, black, attacker));

    // And a creature without fear is blocked as ever.
    let ordinary = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    assert!(can_block(&state, P1, green, ordinary));
}

#[test]
fn intimidate_checks_every_color_pair_and_artifact_exception() {
    use baylee_core::color::ColorSet;
    let colors: Vec<_> = (0..32)
        .map(|mask| {
            ColorSet::from_slice(
                &Color::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(bit, color)| (mask & (1 << bit) != 0).then_some(*color))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 2, 2, KeywordSet::INTIMIDATE);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    for (a, attack_colors) in colors.iter().enumerate() {
        state.object_mut(attacker).unwrap().base_mut().colors = *attack_colors;
        for (b, block_colors) in colors.iter().enumerate() {
            for artifact in [false, true] {
                let base = state.object_mut(blocker).unwrap().base_mut();
                base.colors = *block_colors;
                base.types = if artifact {
                    TypeSet::CREATURE.union(TypeSet::ARTIFACT)
                } else {
                    TypeSet::CREATURE
                };
                assert_eq!(
                    can_block(&state, P1, blocker, attacker),
                    artifact || a & b != 0,
                    "attacker={a}, blocker={b}, artifact={artifact}"
                );
            }
        }
    }
}

/// Landwalk (CR 702.14c) asks the *defending* player's lands: an Island
/// a third player controls does not make an islandwalker unblockable,
/// the defender's own Island does, and so does a land an effect made an
/// Island (the projected type, not the printed one).
#[test]
fn landwalk_reads_the_defending_players_lands_as_they_are() {
    const P2: PlayerId = PlayerId::new(2);
    let mut state = seated(3);
    let walker = creature(&mut state, P0, 2, 2, KeywordSet::ISLANDWALK);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, walker, P1);

    land_of_type(&mut state, P2, land::ISLAND);
    land_of_type(&mut state, P1, land::SWAMP);
    assert!(
        can_block(&state, P1, blocker, walker),
        "a third player's Island and the defender's Swamp change nothing"
    );

    let forest = land_of_type(&mut state, P1, land::FOREST);
    let filter = crate::effects::EffectFilter::object(&state, forest);
    let modifier = baylee_cards_dsl::Modifier::AddSubtype(land::ISLAND);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(forest),
        controller: P1,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 1,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier,
    });
    state.refresh_characteristics();
    assert!(
        !can_block(&state, P1, blocker, walker),
        "a Forest an effect made an Island is an Island"
    );

    // Each walk names its own type: a swampwalker is blocked here.
    let other = creature(&mut state, P0, 2, 2, KeywordSet::MOUNTAINWALK);
    assert!(can_block(&state, P1, blocker, other));
    for (walk, subtype) in LANDWALKS {
        let mut state = empty_state();
        let walker = creature(&mut state, P0, 2, 2, walk);
        let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        assert!(can_block(&state, P1, blocker, walker));
        land_of_type(&mut state, P1, subtype);
        assert!(!can_block(&state, P1, blocker, walker));
    }
}

#[test]
fn shadow_restricts_both_sides_and_does_not_bypass_flying() {
    for attacker_shadow in [false, true] {
        for blocker_shadow in [false, true] {
            for flying in [false, true] {
                let mut state = empty_state();
                let mut attacker_kw = KeywordSet::EMPTY;
                if attacker_shadow {
                    attacker_kw = attacker_kw.union(KeywordSet::SHADOW);
                }
                if flying {
                    attacker_kw = attacker_kw.union(KeywordSet::FLYING);
                }
                let blocker_kw = if blocker_shadow {
                    KeywordSet::SHADOW
                } else {
                    KeywordSet::EMPTY
                };
                let attacker = creature(&mut state, P0, 3, 2, attacker_kw);
                let blocker = creature(&mut state, P1, 2, 2, blocker_kw);
                assert_eq!(
                    can_block(&state, P1, blocker, attacker),
                    attacker_shadow == blocker_shadow && !flying
                );
                let reach = creature(&mut state, P1, 2, 2, blocker_kw.union(KeywordSet::REACH));
                assert_eq!(
                    can_block(&state, P1, reach, attacker),
                    attacker_shadow == blocker_shadow
                );
            }
        }
    }
}

#[test]
fn phased_out_planeswalkers_are_not_defenders() {
    let mut state = empty_state();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let walker = planeswalker(&mut state, p1, 3);
    let defender = Defender::Planeswalker(walker);
    assert!(defender_options(&state, p0).contains(&defender));
    assert_eq!(defending_player(&state, defender), Some(p1));
    state
        .object_mut(walker)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    assert_eq!(defender_options(&state, p0), vec![Defender::Player(p1)]);
    assert_eq!(defending_player(&state, defender), None);
    state
        .object_mut(walker)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    assert!(defender_options(&state, p0).contains(&defender));
}

fn phase_out(state: &mut GameState, id: ObjectId) {
    let mut res = crate::resolve::Resolution {
        source: id,
        on_stack: id,
        controller: state.object(id).unwrap().controller,
        effects: vec![baylee_cards_dsl::Effect::PhaseOut { target: None }],
        pc: 0,
        targets: smallvec::SmallVec::new(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::default(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    let _ = crate::resolve::run(state, &mut res);
    assert!(
        state
            .object(id)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

#[test]
fn phasing_out_an_attacker_removes_it_from_combat() {
    let mut state = empty_state();
    let attacker = creature(&mut state, PlayerId::new(0), 4, 4, KeywordSet::default());
    attack(&mut state, attacker, PlayerId::new(1));
    phase_out(&mut state, attacker);
    assert!(state.combat.attackers().is_empty());
    deal_combat_damage(&mut state, false);
    assert_eq!(state.players[1].life, 20);
    assert!(
        on_battlefield(&state, attacker),
        "phasing is not a zone change"
    );
}

#[test]
fn phasing_out_a_blocker_keeps_the_attacker_blocked() {
    for keywords in [KeywordSet::default(), KeywordSet::TRAMPLE] {
        let mut state = empty_state();
        let attacker = creature(&mut state, PlayerId::new(0), 4, 4, keywords);
        let blocker = creature(&mut state, PlayerId::new(1), 2, 2, KeywordSet::default());
        attack(&mut state, attacker, PlayerId::new(1));
        block(&mut state, blocker, attacker);
        phase_out(&mut state, blocker);
        assert!(state.combat.blockers.is_empty());
        assert!(state.combat.is_blocked(attacker));
        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, attacker), 0);
        assert_eq!(damage(&state, blocker), 0);
        assert_eq!(
            state.players[1].life,
            if keywords.contains(KeywordSet::TRAMPLE) {
                16
            } else {
                20
            }
        );
    }
}

#[test]
fn phasing_out_an_attacked_planeswalker_prevents_damage_and_lifelink() {
    let mut state = empty_state();
    let attacker = creature(&mut state, PlayerId::new(0), 4, 4, KeywordSet::LIFELINK);
    let walker = planeswalker(&mut state, PlayerId::new(1), 5);
    attack_walker(&mut state, attacker, walker);
    phase_out(&mut state, walker);
    deal_combat_damage(&mut state, false);
    assert_eq!(loyalty(&state, walker), 5);
    assert_eq!(state.players[0].life, 20);
    assert_eq!(state.players[1].life, 20);
}
