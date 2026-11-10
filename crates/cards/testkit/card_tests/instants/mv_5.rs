//! The tests of the cards under `cards/instants/mv_5/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// ---- Helpers shared by the tests of this directory -------------------------
//
// A sibling module reaches nothing of this file's neighbours, so the few verbs
// every five-mana spell is played with are written once, here.

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

// oracle_id = "a6a43190-cca2-4f07-afe9-8af681d777da"
fn sedge_troll() -> CardIndex {
    card_index("a6a43190-cca2-4f07-afe9-8af681d777da")
}

/// Seat 0 holds `hand`, with `lands` and `mine` on its side of the table and
/// `theirs` across it, and has reached its own first main phase.
#[track_caller]
fn table(
    seed: u64,
    hand: &[CardIndex],
    lands: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = lands.to_vec();
    board.extend_from_slice(mine);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, me()),
        "seat 0 reaches its own main"
    );
    engine
}

fn hand_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn library_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Library(seat)).len()
}

fn graveyard_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .len()
}

fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Whether `card` is in `seat`'s exile.
fn exiled(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Lets everything on the stack resolve and hands back quiet priority.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| at_rest(e, me()));
}

/// Casts `spell` off every land, names `target` on its menu, resolves it, and
/// returns the menu so the caller can read what was and was not on it.
#[track_caller]
fn cast_at(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    target: ObjectId,
) -> Vec<ObjectId> {
    cast_from_hand(engine, me(), spell);
    let menu = aim_at(engine, me(), target);
    settle(engine);
    menu
}

/// Casts a spell that asks nothing and lets it resolve.
#[track_caller]
fn cast_plain(engine: &mut Engine<RegistryLookup>, spell: CardIndex) {
    cast_from_hand(engine, me(), spell);
    settle(engine);
}

/// Casts `spell`, names `who` among the players, resolves, and returns the
/// menus of objects and of players it offered.
#[track_caller]
fn cast_at_player(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    who: PlayerId,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    cast_from_hand(engine, me(), spell);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, me(), "the caster names the target");
    assert_eq!((min, max), (1, 1), "exactly one target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![who],
            },
        )
        .expect("the named player was on the menu");
    settle(engine);
    (options, player_options)
}

/// Answers every question that comes up while the stack resolves with its
/// first legal answer (discards the first cards offered, keeps a look on
/// top), and returns the questions in the order they were asked.
#[track_caller]
fn settle_asked(engine: &mut Engine<RegistryLookup>) -> Vec<Pending> {
    let mut asked = Vec::new();
    for _ in 0..60 {
        let question = engine.pending().clone();
        match question {
            Pending::Priority { player, .. } => {
                if at_rest(engine, me()) {
                    return asked;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                ref options,
                min,
                ..
            } => {
                let objects = options.iter().copied().take(usize::from(min)).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
                asked.push(question);
            }
            Pending::Arrange {
                player, ref cards, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::Arrange {
                            piles: vec![cards.clone(), vec![]],
                        },
                    )
                    .unwrap();
                asked.push(question);
            }
            other => panic!("unexpected while the spell resolves: {other:?}"),
        }
    }
    panic!("the spell never settled");
}

/// Declares `attacker` against seat 1 and walks to the priority round in which
/// seat 0 is offered the move while the creature is still attacking.
#[track_caller]
fn declare_attack(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(them()))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == me() => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected in the combat phase: {other:?}"),
        }
    }
    panic!("seat 0 never got priority in combat");
}

/// Taps only the mana sources of seat 0 that are `card`.
#[track_caller]
fn tap_only(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    let ids: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == me() && o.card.is_some_and(|c| c.index == card))
        })
        .collect();
    tap_mana_where(engine, me(), |id| ids.contains(&id));
}

/// Puts a Gray Ogre of seat 0 on the stack, paid with the Mountains alone, and
/// returns the spell: a legal target for a counterspell with nothing on the
/// other seat's side of the table.
#[track_caller]
fn ogre_on_the_stack(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    tap_only(engine, mountain());
    cast_with_floating(engine, me(), gray_ogre());
    on_stack(engine, gray_ogre()).expect("the Ogre is on the stack")
}

/// The objects of seat 0 that are tokens.
fn my_tokens(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    tokens_of(engine, me())
}

/// Casts a "choose one" spell with its mode `mode`.
#[track_caller]
fn cast_in_mode(engine: &mut Engine<RegistryLookup>, spell: CardIndex, mode: usize) {
    cast_with_floating(engine, me(), spell);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected the modes, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(mode))
        .unwrap_or_else(|| panic!("mode {mode} was not offered: {options:?}"));
    engine.apply(me(), PlayerAction::ChooseMode(slot)).unwrap();
}
mod annihilate;
mod blessed_light;
mod breath_of_malfegor;
mod bright_reprisal;
mod broken_visage;
mod cinder_cloud;
mod command_the_storm;
mod confirm_suspicions;
mod consume_the_meek;
mod contagion;
mod contradict;
mod dark_nourishment;
mod dark_triumph;
mod desertion;
mod enhanced_awareness;
mod evacuation;
mod fall_of_the_gavel;
mod final_death;
mod final_reward;
mod finishing_blow;
mod fire_and_brimstone;
mod fissure;
mod flesh_to_dust;
mod force_of_will;
mod forge_armor;
mod get_the_point;
mod gush;
mod hatred;
mod humble_the_brute;
mod jace_s_ingenuity;
mod lash_of_the_whip;
mod lava_storm;
mod live_or_die;
mod miraculous_recovery;
mod misdirection;
mod muse_s_encouragement;
mod narset_s_rebuke;
mod necrologia;
mod scars_of_the_veteran;
mod second_thoughts;
mod stonefury;
mod strength_of_cedars;
mod sunder;
mod syphon_fuel;
mod take_up_arms;
mod thoughtflare;
mod throttle;
mod tidy_conclusion;
mod unfathomable_truths;
mod unfriendly_fire;
