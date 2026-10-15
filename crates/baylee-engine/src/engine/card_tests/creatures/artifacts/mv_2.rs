//! The tests of the cards under `cards/creatures/artifacts/mv_2/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Turn one, seat 0 in its first main phase. `hand` and `board` are seat 0's,
/// `theirs` is seat 1's board; the libraries are Forests.
fn table(hand: &[CardIndex], board: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, board)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Casts `card` from seat 0's hand off everything seat 0 can tap and passes
/// until it stands on the battlefield; its enters-trigger is then on the stack.
#[track_caller]
fn cast_and_land(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, card);
    pass_until(engine, |e| on_battlefield(e, p0, card).is_some());
    on_battlefield(engine, p0, card).expect("the spell resolved onto the battlefield")
}

fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

fn zone_count(engine: &Engine<RegistryLookup>, zone: crate::zone::ZoneLocation) -> usize {
    engine.state().zones.list(zone).len()
}

fn hand_count(engine: &Engine<RegistryLookup>, seat: u8) -> usize {
    zone_count(engine, crate::zone::ZoneLocation::Hand(PlayerId::new(seat)))
}

fn graveyard_count(engine: &Engine<RegistryLookup>, seat: u8) -> usize {
    zone_count(
        engine,
        crate::zone::ZoneLocation::Graveyard(PlayerId::new(seat)),
    )
}

/// Passes until a surveil of `n` is asked of seat 0 and hands back the cards
/// it looks at, having checked the question is a surveil of that size.
#[track_caller]
fn surveil_asked(engine: &mut Engine<RegistryLookup>, n: u32) -> Vec<ObjectId> {
    pass_until(engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the Arrange question")
    };
    assert_eq!(player, PlayerId::new(0), "the controller surveils");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(n), "surveil {n}: top, then graveyard");
    assert_eq!(cards.len(), n as usize, "it looks at the top {n}");
    cards
}

/// Seat 0 attacks seat 1 with `attacker`, unblocked, and the game runs to the
/// second main phase.
#[track_caller]
fn swing_unblocked(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    attack_and_collect_blocks(engine, attacker, PlayerId::new(1));
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .expect("no blocks");
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// A vanilla artifact creature: cast from hand off `lands`, it resolves
/// onto the battlefield as an artifact creature with the printed body, no
/// keywords and nothing left on the stack. Hands the table back for more.
#[track_caller]
fn vanilla_cast(
    card: CardIndex,
    lands: &[CardIndex],
    body: (i16, i16),
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[card], lands, &[]);
    assert!(in_hand(&engine, p0, card).is_some(), "it starts in hand");
    let creature = cast_and_land(&mut engine, card);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, card).is_none(), "it left the hand");
    assert_eq!(pt(&engine, creature), body, "the printed body");
    let t = types(&engine, creature);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert_eq!(
        keywords(&engine, creature),
        KeywordSet::EMPTY,
        "no keywords"
    );
    (engine, creature)
}

/// A vanilla artifact creature seated from the start attacks unblocked and
/// deals exactly its power.
#[track_caller]
fn vanilla_swing(card: CardIndex, power: i32) {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[card], &[]);
    let creature = on_battlefield(&engine, p0, card).expect("seated");
    swing_unblocked(&mut engine, creature);
    assert_eq!(life_of(&engine, 1), 20 - power);
    assert_eq!(life_of(&engine, 0), 20);
}

/// Whether ability `index` of `source` is on offer at priority. A printed mana
/// ability is an ordinary entry of the offer's `abilities`.
fn offered(engine: &Engine<RegistryLookup>, source: ObjectId, index: u32) -> bool {
    priority_offer(engine).abilities.contains(&(source, index))
}

/// Answers "add one mana of any color" with `color`, after checking the
/// question offers the five colors and not colorless.
#[track_caller]
fn name_a_color(engine: &mut Engine<RegistryLookup>, color: ManaColor) {
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, PlayerId::new(0), "the controller names it");
    assert_eq!(
        options.len(),
        5,
        "five colors, colorless is none: {options:?}"
    );
    assert!(!options.contains(&ManaColor::Colorless));
    engine
        .apply(player, PlayerAction::ChooseColor(color))
        .expect("the color was on the menu");
}

/// "{1}: Add one mana of any color. Activate only once each turn." as
/// printed on a creature seated beside two Forests: unpayable with nothing
/// floating, one mana of the named color for one, a second activation the
/// same turn refused however much is floating, and available again the
/// next turn.
#[track_caller]
fn once_each_turn_any_color(card: CardIndex) {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[card, forest(), forest()], &[]);
    let me = on_battlefield(&engine, p0, card).expect("seated");
    let forests = all_on_battlefield(&engine, p0, forest());
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(me, 0));
    assert!(!offered(&engine), "nothing floating to pay {{1}} with");

    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    assert!(offered(&engine), "one mana floating pays the {{1}}");
    activate(&mut engine, p0, card, 0);
    name_a_color(&mut engine, ManaColor::Blue);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "the named color");
    assert_eq!(pool.total(), 1, "the {{1}} was spent");
    assert!(!is_tapped(&engine, me), "its price is mana, not a tap");

    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!offered(&engine), "only once each turn");

    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    assert!(offered(&engine), "a new turn, a new activation");
}

mod a_i_m_synthoids;
mod alpha_myr;
mod arcbound_ravager;
mod baleful_strix;
mod battering_ram;
mod bronze_sable;
mod buzz_bots;
mod coastal_bulwark;
mod consulate_skygate;
mod copper_myr;
mod coretapper;
mod doom_s_servo_guards;
mod field_creeper;
mod friendly_teddy;
mod gleaming_barrier;
mod gold_myr;
mod haunted_guardian;
mod hedron_crawler;
mod hovermyr;
mod iron_myr;
mod leaden_myr;
mod malcator_s_watcher;
mod manakin;
mod mandroid_squadron;
mod myr_retriever;
mod myr_sire;
mod omega_myr;
mod ornithopter_of_paradise;
mod perilous_myr;
mod piggy_bank;
mod pili_pala;
mod prismite;
mod runed_servitor;
mod salvaged_manaworker;
mod sanitation_automaton;
mod scarecrow_guide;
mod shire_scarecrow;
mod silver_myr;
mod sundial_dawn_tyrant;
mod three_tree_mascot;
mod tidehollow_strix;
mod vault_skirge;
mod vedalken_outlander;
mod wreckage_wickerfolk;
mod wretched_doll;
