//! The tests of the cards under `cards/creatures/mv_7/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod resolute_archangel;

// ---------------------------------------------------------------------------
// Helpers shared by the generated seven-drops. Each reads one thing the
// per-card tests would otherwise spell out again.

/// `n` copies of `card`, for a board of basic lands.
fn lots(card: CardIndex, n: usize) -> Vec<CardIndex> {
    vec![card; n]
}

/// Seat 0 in its first main phase with `board` out and `hand` in hand; seat 1
/// has `theirs` on its battlefield. The library is Forests.
fn table7(board: &[CardIndex], hand: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, hand)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Whether the engine offers casting `card` from seat 0's hand right now.
fn castable7(engine: &Engine<RegistryLookup>, card: CardIndex) -> bool {
    let id = in_hand(engine, PlayerId::new(0), card).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.castable.contains(&id)
}

/// Pays for `card` with every land seat 0 has, which the caller sized to the
/// printed cost exactly: with one land left untapped the pool is one short
/// and the offer is withheld; with the last one tapped it is offered and the
/// cast goes on the stack.
#[track_caller]
fn pay_exactly_and_cast(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    let p0 = PlayerId::new(0);
    let lands = lands_of(engine, p0);
    let (last, rest) = lands.split_last().expect("lands to pay with");
    tap_mana_where(engine, p0, |id| rest.contains(&id));
    assert!(
        !castable7(engine, card),
        "one mana short of the printed cost is not enough"
    );
    tap_mana_where(engine, p0, |id| id == *last);
    assert!(
        castable7(engine, card),
        "the printed cost is in the pool: the cast is offered"
    );
    cast_with_floating(engine, p0, card);
}

/// Casts `card` off exactly its cost and lets it and whatever it triggers
/// resolve, for a card whose triggers ask nothing. Returns the permanent.
#[track_caller]
fn cast_and_resolve(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    pay_exactly_and_cast(engine, card);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, PlayerId::new(0), card)
        .expect("the creature resolved onto the battlefield")
}

/// Seat 0 attacks seat 1 with `attackers`; seat 1 blocks as `blocks` lists
/// `(blocker, attacker)`; combat runs through to the second main phase.
#[track_caller]
fn fight(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) {
    declare_attack(engine, attackers, blocks);
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// The first half of [`fight`]: the attack and the blocks are declared and
/// the game stops right after, for a test that wants to stop on a question
/// the combat damage asks.
#[track_caller]
fn declare_attack(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) {
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
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the blocks are legal");
}

/// Answers the pending target choice with `objects` and `players`.
#[track_caller]
fn point_at(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: objects.to_vec(),
                players: players.to_vec(),
            },
        )
        .expect("the target is on the menu");
}

/// The options of the pending target choice, asserting there is one.
#[track_caller]
fn target_menu(engine: &Engine<RegistryLookup>) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    (options, player_options)
}

/// The printed body and keywords of a permanent, as the layers project them.
#[track_caller]
fn assert_body(
    engine: &Engine<RegistryLookup>,
    id: ObjectId,
    body: (i16, i16),
    printed: KeywordSet,
) {
    assert_eq!(pt(engine, id), body, "the printed power and toughness");
    assert!(
        printed == KeywordSet::default() || keywords(engine, id).contains(printed),
        "the printed keywords are on the permanent"
    );
}

/// Seat 0 in its first main phase with `card` (and `more`) already on its
/// battlefield, a Gray Ogre on seat 1's: `(engine, the card, the Ogre)`.
#[track_caller]
fn duel_vs_ogre(
    card: CardIndex,
    more: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![card];
    board.extend_from_slice(more);
    let engine = table7(&board, &[], &[gray_ogre()]);
    let mine = on_battlefield(&engine, p0, card).expect("the card is out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the Ogre is out");
    (engine, mine, ogre)
}

/// Whether seat 1's Gray Ogre may block `card` when seat 0 attacks with `card`
/// and a Gray Ogre of its own: `(may block the card, may block the other
/// Ogre)`. The second answer is the control, true for any card whose text
/// says nothing of evasion.
#[track_caller]
fn ogre_may_block(card: CardIndex) -> (bool, bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[card, gray_ogre()], &[], &[gray_ogre()]);
    let mine = on_battlefield(&engine, p0, card).expect("the card is out");
    let my_ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre is out");
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (mine, Defender::Player(p1)),
                    (my_ogre, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    let may = |attacker: ObjectId| {
        blockers
            .iter()
            .any(|b| b.blocker == their_ogre && b.attackers.contains(&attacker))
    };
    (may(mine), may(my_ogre))
}

/// Whether `card` (a reach creature) and seat 0's Gray Ogre may block a
/// flier attacking on seat 1's turn: `(the card may, the Ogre may)`.
#[track_caller]
fn reach_may_block(card: CardIndex) -> (bool, bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, gray_ogre()])
        .battlefield(1, &[flier()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, card).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    let blocks = their_attack_blocks(&mut engine, sphinx);
    let can = |who: ObjectId| {
        blocks
            .iter()
            .any(|b| b.blocker == who && b.attackers.contains(&sphinx))
    };
    (can(mine), can(ogre))
}

/// Cycles `card` out of seat 0's hand with `lands` on its battlefield, the
/// ability being printed ability `ability`: one mana short withholds the
/// ability, the full price offers it, and cycling leaves the card in the
/// graveyard with one card drawn in its place.
#[track_caller]
fn cycle_it(card: CardIndex, lands: &[CardIndex], ability: u32) {
    let p0 = PlayerId::new(0);
    let mut engine = table7(lands, &[card], &[]);
    let id = in_hand(&engine, p0, card).expect("the card is in hand");
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(id, ability));
    let lands = lands_of(&engine, p0);
    let (last, rest) = lands.split_last().expect("lands");
    tap_mana_where(&mut engine, p0, |l| rest.contains(&l));
    assert!(!offered(&engine), "one short of the cycling cost");
    tap_mana_where(&mut engine, p0, |l| l == *last);
    assert!(offered(&engine), "the cycling cost is in the pool");
    let library = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, card, ability);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, card).is_none(),
        "the card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, card).is_some(),
        "it was discarded"
    );
    assert_eq!(library_size(&engine, p0), library - 1, "one card drawn");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "the drawn card replaced the cycled one"
    );
}

/// Isamaru, Hound of Konda: a legendary 2/2 with no rules text, the quiet
/// legendary creature.
fn isamaru() -> CardIndex {
    card_index("cfb756dd-6fe1-430a-8f04-8b10cac62a86")
}

/// An 8/7 flier for the other side of the table, Goliath Sphinx.
fn flier() -> CardIndex {
    card_index("73115500-4468-43b3-9151-13e3da9050c5")
}

/// Seat 1 attacks seat 0 with `attacker` on seat 1's own turn and the blocks
/// seat 0 is offered come back; the game stays at the declare-blockers
/// question.
#[track_caller]
fn their_attack_blocks(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
) -> Vec<crate::choice::BlockOption> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .expect("the attacker may attack");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    blockers
}

/// Seat 0 blocks as `blocks` lists `(blocker, attacker)` at the pending
/// declare-blockers question and combat runs to the second main phase.
#[track_caller]
fn block_and_finish(engine: &mut Engine<RegistryLookup>, blocks: &[(ObjectId, ObjectId)]) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the blocks are legal");
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// What Lightning Bolt cast by seat `caster` may aim at when seat 0 has `card`
/// and a Gray Ogre in play: `(menu, the card's permanent, the Ogre)`. The Ogre
/// is the control that shows the menu is not simply empty.
#[track_caller]
fn bolt_menu(card: CardIndex, caster: usize) -> (Vec<ObjectId>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let seat = if caster == 0 { p0 } else { p1 };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, gray_ogre(), mountain()])
        .battlefield(1, &[mountain()])
        .hand(caster, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    if caster == 0 {
        reach_main_phase(&mut engine, p0);
    } else {
        reach_their_main_phase(&mut engine, p1);
    }
    let permanent = on_battlefield(&engine, p0, card).expect("the card is out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre is out");
    cast_from_hand(&mut engine, seat, lightning_bolt());
    let (options, _) = target_menu(&engine);
    (options, permanent, ogre)
}

/// Seat `seat`'s life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

mod akroma_vision_of_ixidor;
mod amugaba;
mod ancestor_s_chosen;
mod angel_of_despair;
mod angel_of_retribution;
mod angel_of_serenity;
mod angel_of_the_dire_hour;
mod angler_turtle;
mod archangel;
mod archweaver;
mod ashen_monstrosity;
mod atraxa_grand_unifier;
mod aven_fateshaper;
mod axebane_stag;
mod birthing_hulk;
mod black_dragon;
mod bladewing_the_risen;
mod blizzard_elemental;
mod boldwyr_intimidator;
mod bruna_the_fading_light;
mod cat_gator;
mod cateran_overlord;
mod chaos_lord;
mod charnelhoard_wurm;
mod craw_giant;
mod crowd_favorites;
mod delraich;
mod djinn_of_the_lamp;
mod doc_ock_evil_inventor;
mod dragon_mage;
mod drogskol_cavalry;
mod drowner_of_truth;
mod duskdale_wurm;
mod ebon_dragon;
mod elder_land_wurm;
mod elesh_norn_grand_cenobite;
mod enormous_baloth;
mod fleshpulper_giant;
mod flora_colossus;
mod fungal_shambler;
mod gabriel_angelfire;
mod gargantuan_gorilla;
mod ghastbark_twins;
mod goblin_dynamo;
mod goliath_sphinx;
mod great_whale;
mod greater_sandwurm;
mod groundshaker_sliver;
mod guardian_of_cloverdell;
mod havenwood_wurm;
mod havoc_demon;
mod hazezon_tamar;
mod hellkite_igniter;
mod hellkite_punisher;
mod hornet_queen;
mod howling_giant;
mod hundroog;
mod island_fish_jasconius;
mod jin_gitaxias_progress_tyrant;
mod jumbo_cactuar;
mod jungle_weaver;
mod kalonian_behemoth;
mod karplusan_giant;
mod kazarov_sengir_pureblood;
mod krosan_groundshaker;
mod laccolith_titan;
mod loamdragger_giant;
mod lord_of_the_pit;
mod luminous_angel;
mod macetail_hystrodon;
mod magma_giant;
mod marjhan;
mod maul_splicer;
mod minion_of_leshrac;
mod minion_of_tevesh_szat;
mod minotaur_aggressor;
mod molimo_maro_sorcerer;
mod mossbeard_ancient;
mod nebula_dragon;
mod oakgnarl_warrior;
mod omnath_locus_of_rage;
mod oyobi_who_split_the_heavens;
mod palinchron;
mod pelakka_wurm;
mod penumbra_wurm;
mod pit_spawn;
mod plated_crusher;
mod plated_slagwurm;
mod primoc_escapee;
mod quilled_slagwurm;
mod rampaging_brontodon;
mod ridge_rannet;
mod risen_sanctuary;
mod riven_turnbull;
mod roaring_slagwurm;
mod rockcaster_platoon;
mod rune_scarred_demon;
mod sephara_sky_s_blade;
mod seraph;
mod seraph_of_the_suns;
mod serra_s_emissary;
mod shaleskin_bruiser;
mod shauku_endbringer;
mod shivan_hellkite;
mod shrouded_serpent;
mod silent_sentinel;
mod simic_sky_swallower;
mod skyshroud_behemoth;
mod slipstream_eel;
mod spearbreaker_behemoth;
mod sphinx_mindbreaker;
mod sphinx_of_the_final_word;
mod spike_hatcher;
mod spined_megalodon;
mod stone_tongue_basilisk;
mod striped_riverwinder;
mod the_balrog_of_moria;
mod the_terror_of_serpent_s_pass;
mod thorn_elemental;
mod thriss_nantuko_primus;
mod thunder_dragon;
mod thundercloud_elemental;
mod thundering_spineback;
mod tolarian_serpent;
mod trained_orgg;
mod trolls_of_tel_jilad;
mod trostani_s_summoner;
mod tsabo_tavoc;
mod tyrranax_rex;
mod vagrant_plowbeasts;
mod veldrane_of_sengir;
mod venerable_lammasu;
mod vizzerdrix;
mod whiptail_wurm;
mod windbrisk_raptor;
mod windreader_sphinx;
mod wormfang_manta;
mod yoked_plowbeast;
mod zhou_yu_chief_commander;
