//! The tests of the cards under `cards/creatures/artifacts/mv_3/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seat 0 holds priority in its first main phase with `hand` in hand and
/// `board` on the battlefield (seated unsick, untapped).
fn main_with(hand: &[CardIndex], board: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Casts `card` for seat 0 off everything that makes mana, then lets the spell
/// and whatever it triggered resolve, up to the next priority with an empty
/// stack. A test whose trigger asks something stops before this.
#[track_caller]
fn cast_and_settle(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    cast_from_hand(engine, PlayerId::new(0), card);
    pass_until(engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
}

/// The library of `seat`, bottom first: the last entry is the top card.
fn library_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

/// A seat's life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

/// Walks to the scry question, checks it is seat 0's scry over the top `n` of
/// its library, bottoms the whole look when `bottom_all` and else keeps it,
/// and settles. Returns the library from before the scry, bottom first, so the
/// caller can check where the looked-at cards ended up.
#[track_caller]
fn answer_scry(engine: &mut Engine<RegistryLookup>, n: usize, bottom_all: bool) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    let before = library_of(engine, p0);
    pass_until(engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the controller does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    let looked: Vec<ObjectId> = before.iter().rev().take(n).copied().collect();
    assert_eq!(cards, looked, "the top {n} cards, top first");
    assert_eq!(piles, scry_piles(u32::try_from(n).unwrap()));
    let away: Vec<ObjectId> = if bottom_all { cards.clone() } else { vec![] };
    engine
        .apply(p0, look_answer(&cards, &away))
        .expect("a scry answer");
    pass_until(engine, |e| at_rest(e, p0));
    before
}

/// A scry of `n` that bottoms the whole look must leave those cards at the
/// bottom, and the library the length it was; one that
/// keeps it must leave the library untouched. `before` is what
/// [`answer_scry`] returned.
#[track_caller]
fn assert_scried(engine: &Engine<RegistryLookup>, before: &[ObjectId], n: usize, bottomed: bool) {
    let after = library_of(engine, PlayerId::new(0));
    assert_eq!(after.len(), before.len(), "scry draws nothing");
    if bottomed {
        let looked: Vec<ObjectId> = before.iter().rev().take(n).copied().collect();
        for card in &looked {
            assert!(after[..n].contains(card), "the look lies at the bottom");
        }
        assert_eq!(
            after.last(),
            before.iter().rev().nth(n),
            "the card below the look is the new top"
        );
    } else {
        assert_eq!(after, before, "keeping everything moves nothing");
    }
}

/// Declares `attackers` against seat 1 and stops where seat 1 is asked to
/// block, returning what seat 1 may block with.
#[track_caller]
fn attack_to_blocks(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
) -> Vec<crate::choice::BlockOption> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    blockers
}

/// Whether `blocker` may block `attacker` according to the offered options.
fn may_block(
    options: &[crate::choice::BlockOption],
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    options
        .iter()
        .any(|o| o.blocker == blocker && o.attackers.contains(&attacker))
}

/// A board for evasion: seat 0 has `card` and a plain ground creature, seat 1
/// has `their_board`. Seat 0 attacks with both; returns the engine standing
/// at seat 1's block question, the two attackers (`card`'s object first) and
/// what seat 1 may block with.
#[track_caller]
fn evasion_probe(
    card: CardIndex,
    their_board: &[CardIndex],
) -> (
    Engine<RegistryLookup>,
    [ObjectId; 2],
    Vec<crate::choice::BlockOption>,
) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, quiet_creature()])
        .battlefield(1, their_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let subject = on_battlefield(&engine, p0, card).expect("seated");
    let ground = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let options = attack_to_blocks(&mut engine, &[subject, ground]);
    (engine, [subject, ground], options)
}

/// What seat 0 is offered as attackers at its next declare-attackers step.
#[track_caller]
fn attackers_offered(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    attackers
}

/// Taps `source` for its "one mana of any color" ability by hand: the ability
/// stops to ask which, the question offers all five colors, and answering
/// `color` leaves exactly one mana of it in seat 0's pool and `source` tapped.
#[track_caller]
fn tap_for_any_color(engine: &mut Engine<RegistryLookup>, source: ObjectId, color: ManaColor) {
    let p0 = PlayerId::new(0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(!is_tapped(engine, source));
    let before = engine.state().players[0].mana_pool.total();
    // A mana ability that asks a question is offered among the abilities.
    let action = if legal.mana_abilities.contains(&source) {
        PlayerAction::ActivateManaAbility { source }
    } else {
        let (_, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(s, _)| *s == source)
            .expect("its mana ability is offered");
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    engine
        .apply(p0, action)
        .expect("the mana ability activates");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("it asks which color, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    for c in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(options.contains(&c), "{c:?} is among the colors offered");
    }
    engine
        .apply(p0, PlayerAction::ChooseColor(color))
        .expect("a color offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(color), 1, "one mana of the color named");
    assert_eq!(pool.total(), before + 1, "and only one mana");
    assert!(is_tapped(engine, source), "{{T}} is the cost");
}

/// Answers the pending target choice with the one player `who`, returning the
/// players the choice offered.
#[track_caller]
fn aim_at_player(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    who: PlayerId,
) -> Vec<PlayerId> {
    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "the seat that controls it names the target");
    assert_eq!((min, max), (1, 1), "one target");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![who],
            },
        )
        .expect("the named player was on the menu");
    player_options
}

/// Walks to seat 1's first main phase and returns the length of seat 1's
/// library there.
#[track_caller]
fn p1_library_at_its_main(engine: &mut Engine<RegistryLookup>) -> usize {
    let p1 = PlayerId::new(1);
    reach_their_main_phase(engine, p1);
    library_of(engine, p1).len()
}
mod a_i_m_bot;
mod aesthir_glider;
mod alloy_myr;
mod assembly_worker;
mod automatic_librarian;
mod blood_servitor;
mod bottle_gnomes;
mod brass_secretary;
mod bronze_walrus;
mod cathodion;
mod chrome_cat;
mod chrome_prowler;
mod coiled_tinviper;
mod custodian_of_the_trove;
mod darksteel_myr;
mod dragon_engine;
mod elf_replica;
mod ersatz_gnomes;
mod esper_battlemage;
mod esperzoa;
mod etched_familiar;
mod etherium_spinner;
mod exocomp;
mod farmstead_gleaner;
mod filigree_familiar;
mod flayed_one;
mod goblin_replica;
mod guardians_of_meletis;
mod hoof_skulkin;
mod hopping_automaton;
mod horrible_hordes;
mod hotfoot_gnome;
mod iron_bully;
mod jangling_automaton;
mod junk_diver;
mod kiln_walker;
mod machinesmith_automaton;
mod marble_gargoyle;
mod metalworker;
mod moonfolk_puzzlemaker;
mod moriok_replica;
mod myr_adapter;
mod myr_landshaper;
mod neurok_replica;
mod night_market_guard;
mod nim_replica;
mod oasis_gardener;
mod one_eyed_scarecrow;
mod onulet;
mod opaline_unicorn;
mod palladium_myr;
mod parasitic_strix;
mod patchwork_gnomes;
mod phyrexian_war_beast;
mod porcelain_legionnaire;
mod prakhata_pillar_bug;
mod redrock_sentinel;
mod reinforced_bulwark;
mod rune_sealed_wall;
mod s_h_i_e_l_d_deployment_drone;
mod sanctuary_raptor;
mod sarcomite_myr;
mod serrated_biskelion;
mod skyscanner;
mod sliver_construct;
mod soldier_replica;
mod sparring_golem;
mod spined_thopter;
mod steel_golem;
mod sterling_hound;
mod stonework_puma;
mod sylvok_replica;
mod tezzeret_s_strider;
mod thaumaturge_s_familiar;
mod thermal_navigator;
mod ticking_gnomes;
mod transmogrifying_licid;
mod vectis_silencers;
mod walking_archive;
mod wall_of_shields;
mod wall_of_spears;
mod watchdog;
mod watcher_of_the_wayside;
mod watchful_automaton;
mod weldfast_monitor;
mod wicker_witch;
mod wizard_replica;
mod yotian_soldier;
