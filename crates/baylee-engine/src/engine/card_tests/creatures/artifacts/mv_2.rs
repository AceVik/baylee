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

/// Seat 0's `card` attacks into a Bears and a Storm Crow: the Crow may block
/// it, the Bears (no flying, no reach) may not.
#[track_caller]
fn only_a_flier_may_block(card: CardIndex) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[card], &[grizzly_bears(), storm_crow()]);
    let me = on_battlefield(&engine, p0, card).expect("seated");
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, me, p1);
    assert!(
        options.iter().any(|o| o.blocker == crow),
        "a flier may block"
    );
    assert!(
        !options.iter().any(|o| o.blocker == bears),
        "a ground creature may not"
    );
}

/// Passes to seat 0's next turn, first main phase.
#[track_caller]
fn to_my_next_turn(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let now = engine.state().turn.number;
    pass_until(engine, |e| {
        e.state().turn.number > now
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

/// Answers whatever stands between here and the next quiet priority, naming
/// seat 1 for a "target opponent"/"any target" question, and says whether such
/// a question was asked.
#[track_caller]
fn settle_aiming_at_opponent(engine: &mut Engine<RegistryLookup>) -> bool {
    let p1 = PlayerId::new(1);
    let mut asked = false;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                asked = true;
                assert!(options.contains(&p1), "the opponent is on the menu");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .expect("the opponent is a legal choice");
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                asked = true;
                assert!(player_options.contains(&p1), "the opponent is on the menu");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the opponent is a legal target");
            }
            Pending::Priority { .. } if stack_is_empty(engine) => return asked,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }
    panic!("no quiet priority within forty actions")
}

/// Seat 0's `attacker` attacks seat 1 and the combat runs on. The blocks the
/// opponent is then offered (`Some`), or `None` when nobody could block and
/// the question was never asked.
#[track_caller]
fn attack_block_options(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
) -> Option<Vec<crate::choice::BlockOption>> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the attacker was on the offer");
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => return Some(blockers),
            Pending::Priority { player, .. } => {
                if engine.state().turn.step == Step::CombatDamage
                    || engine.state().turn.phase == Phase::SecondMain
                {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached the blockers or the damage")
}

mod a_i_m_synthoids;
mod aegis_automaton;
mod alpha_myr;
mod arcbound_ravager;
mod attentive_sunscribe;
mod baleful_strix;
mod battering_ram;
mod bronze_sable;
mod buzz_bots;
mod coastal_bulwark;
mod consulate_skygate;
mod copper_gnomes;
mod copper_myr;
mod coretapper;
mod corridor_monitor;
mod darklit_gargoyle;
mod diversion_unit;
mod doom_s_servo_guards;
mod escaped_experiment;
mod field_creeper;
mod friendly_teddy;
mod glaze_fiend;
mod gleaming_barrier;
mod gold_myr;
mod gust_skimmer;
mod haunted_guardian;
mod hedron_crawler;
mod hedron_scrabbler;
mod hermitic_nautilus;
mod hovermyr;
mod hydra_assault_robot;
mod iron_myr;
mod jhoira_s_toolbox;
mod jousting_dummy;
mod leaden_myr;
mod malcator_s_watcher;
mod manakin;
mod mandible_justiciar;
mod mandroid_squadron;
mod mechan_navigator;
mod mechanized_ninja_cavalry;
mod metallurgeon;
mod monoskelion;
mod myr_retriever;
mod myr_sire;
mod narnam_cobra;
mod omega_myr;
mod one_clown_band;
mod ornithopter_of_paradise;
mod parcel_myr;
mod perilous_myr;
mod piggy_bank;
mod pili_pala;
mod prismite;
mod ragged_playmate;
mod red_herring;
mod runed_servitor;
mod salvaged_manaworker;
mod sanitation_automaton;
mod scarecrow_guide;
mod scorpion_sentinel;
mod shire_scarecrow;
mod silver_myr;
mod sliversmith;
mod sundial_dawn_tyrant;
mod suspicious_bookcase;
mod three_tree_mascot;
mod tidehollow_strix;
mod tough_cookie;
mod towering_viewpoint;
mod toymaker;
mod vault_skirge;
mod vedalken_outlander;
mod verdant_automaton;
mod voltaic_servant;
mod wall_of_junk;
mod wall_of_tanglecord;
mod welder_automaton;
mod wreckage_wickerfolk;
mod wretched_doll;
