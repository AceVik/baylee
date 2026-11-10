//! The tests of the cards under `cards/sorceries/mv_6/`, one module per card.

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
    table_with_their_hand(seed, hand, lands, mine, theirs, &[])
}

/// The same table, with `their_hand` dealt to seat 1 as well.
#[track_caller]
fn table_with_their_hand(
    seed: u64,
    hand: &[CardIndex],
    lands: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
    their_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = lands.to_vec();
    board.extend_from_slice(mine);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .hand(0, hand)
        .hand(1, their_hand)
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
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            min,
            max,
            ..
        } => {
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
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, me(), "the caster names the target");
            engine
                .apply(me(), PlayerAction::ChoosePlayer(who))
                .expect("the named player was on the menu");
            settle(engine);
            (vec![], options)
        }
        other => panic!("expected a target choice, got {other:?}"),
    }
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

/// Casts `spell`, names `target` on its menu, lets it resolve and answers what
/// is asked on the way; returns the menu and the questions.
#[track_caller]
fn cast_at_asked(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    target: ObjectId,
) -> (Vec<ObjectId>, Vec<Pending>) {
    cast_from_hand(engine, me(), spell);
    let menu = aim_at(engine, me(), target);
    (menu, settle_asked(engine))
}

/// Casts `spell`, names `who` among the players, lets it resolve and answers
/// what is asked on the way; returns both menus and the questions.
#[track_caller]
fn cast_at_player_asked(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    who: PlayerId,
) -> (Vec<ObjectId>, Vec<PlayerId>, Vec<Pending>) {
    cast_from_hand(engine, me(), spell);
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            min,
            max,
            ..
        } => {
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
            (options, player_options, settle_asked(engine))
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, me(), "the caster names the target");
            engine
                .apply(me(), PlayerAction::ChoosePlayer(who))
                .expect("the named player was on the menu");
            (vec![], options, settle_asked(engine))
        }
        other => panic!("expected a target choice, got {other:?}"),
    }
}

/// Walks to the scry or surveil question, checks it is seat 0's, and answers
/// it by sending the first `away` of the looked-at cards to the second pile.
/// Returns the cards looked at.
#[track_caller]
fn look_and_send(engine: &mut Engine<RegistryLookup>, away: usize) -> Vec<ObjectId> {
    pass_until(engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, me(), "the caster looks");
    engine
        .apply(me(), look_answer(&cards, &cards[..away]))
        .expect("the answer came out of the question");
    cards
}

/// How many permanents are on the battlefield.
fn battlefield_len(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Battlefield).len()
}
mod akroma_s_vengeance;
mod amnesia;
mod assassin_s_strike;
mod austere_command;
mod bad_deal;
mod boon_of_the_wish_giver;
mod catastrophe;
mod certain_death;
mod collective_unconscious;
mod corrupt;
mod crisis_of_conscience;
mod dark_offering;
mod defibrillating_current;
mod desert_twister;
mod devils_playground;
mod essence_feed;
mod extinguish_all_hope;
mod farewell;
mod fiery_finish;
mod grisly_ritual;
mod guan_yu_s_1_000_li_march;
mod icatian_town;
mod imperial_oath;
mod jokulhaups;
mod kirtar_s_wrath;
mod kiss_of_the_amesha;
mod mass_production;
mod mindculling;
mod one_dozen_eyes;
mod planar_cleansing;
mod poison_arrow;
mod pulverize;
mod rapturous_moment;
mod rise_of_eagles;
mod sangrite_surge;
mod sip_of_hemlock;
mod spectral_procession;
mod thassa_s_bounty;
mod their_name_is_death;
mod time_spiral;
mod vanquish_the_foul;
mod zof_consumption;
