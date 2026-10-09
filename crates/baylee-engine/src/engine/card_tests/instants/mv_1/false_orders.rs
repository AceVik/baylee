//! `cards/instants/mv_1/false_orders.rs`, played.
//!
//! "Cast this spell only during the declare blockers step. Remove target
//! creature defending player controls from combat. Creatures it was blocking
//! that had become blocked by only that creature this combat become
//! unblocked. You may have it block an attacking creature of your choice."
//!
//! The scenarios drive every question by hand: `pass_until` would answer
//! nothing of the re-block (it panics on one), so the decline paths run for
//! real, as the choice `ChoicePrompt::BlockWith` the caster is asked.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn false_orders() -> CardIndex {
    card_index("38c5c952-8153-4d98-89b5-a75260383345")
}

/// `{3}{R}` 3/3 red Giant.
fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `{G}` 1/1: "Whenever this creature blocks or becomes blocked by a
/// creature, that creature becomes green."
fn aisling_leprechaun() -> CardIndex {
    card_index("5456f00c-0bef-4c14-902f-f5c14475f284")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn colors_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> ColorSet {
    engine
        .state()
        .object(object)
        .unwrap()
        .characteristics()
        .colors
}

fn red() -> ColorSet {
    ColorSet::from_slice(&[Color::Red])
}

fn green() -> ColorSet {
    ColorSet::from_slice(&[Color::Green])
}

fn sorted(mut objects: Vec<ObjectId>) -> Vec<ObjectId> {
    objects.sort();
    objects
}

/// Every `card` on the battlefield under `seat`'s control, for a board with
/// two of a kind.
fn all_on_battlefield(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// A duel in which both seats have a Mountain, a Hill Giant and a Serra
/// Angel on the battlefield and a False Orders in hand.
fn mirrored(seed: u64) -> Engine<RegistryLookup> {
    let board = [mountain(), hill_giant(), serra_angel()];
    let mut engine = Duel::new(seed, mountain())
        .battlefield(0, &board)
        .battlefield(1, &board)
        .hand(0, &[false_orders()])
        .hand(1, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// `active` declares `attacks`, and priority comes round to `active` in the
/// declare attackers step.
#[track_caller]
fn declare_attackers(
    engine: &mut Engine<RegistryLookup>,
    active: PlayerId,
    attacks: &[(ObjectId, Defender)],
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == active),
    );
    engine
        .apply(
            active,
            PlayerAction::DeclareAttackers {
                attackers: attacks.to_vec(),
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == active),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, Step::DeclareAttackers);
}

/// Every defending seat declares its share of `blocks` (`(blocker,
/// attacker)`), and `active` holds priority in the declare blockers step.
#[track_caller]
fn declare_blocks(
    engine: &mut Engine<RegistryLookup>,
    active: PlayerId,
    blocks: &[(ObjectId, ObjectId)],
) {
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { player, .. } => {
                let blockers = blocks
                    .iter()
                    .copied()
                    .filter(|&(b, _)| engine.state().object(b).unwrap().controller == player)
                    .collect();
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers })
                    .unwrap();
            }
            Pending::Priority { player, .. }
                if player == active && engine.state().turn.step == Step::DeclareBlockers =>
            {
                return;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("on the way to the declare blockers step: {other:?}"),
        }
    }
    panic!(
        "the declare blockers step never came: {:?}",
        engine.pending()
    );
}

/// `declare_attackers`, then `declare_blocks`.
#[track_caller]
fn to_blocks(
    engine: &mut Engine<RegistryLookup>,
    active: PlayerId,
    attacks: &[(ObjectId, Defender)],
    blocks: &[(ObjectId, ObjectId)],
) {
    declare_attackers(engine, active, attacks);
    declare_blocks(engine, active, blocks);
}

/// Whether the engine offers the False Orders in `seat`'s hand to `seat`
/// right now, with every mana source of theirs tapped for it (floating mana
/// is what an offer is read against, and it empties between steps), and
/// whether a cast is accepted (which casts it: the caller is done with the
/// game). The two have to agree.
#[track_caller]
fn castable_now(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    tap_all_mana(engine, seat);
    let card = in_hand(engine, seat, false_orders()).expect("the spell is in hand");
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(player, seat);
    let offered = legal.castable.contains(&card);
    let accepted = engine.apply(seat, PlayerAction::CastSpell { card }).is_ok();
    assert_eq!(offered, accepted, "the offer and the engine disagree");
    offered
}

/// `caster` casts False Orders (priority first coming round to them if the
/// active player holds it), and the targets the spell offers are returned.
#[track_caller]
fn cast_and_offered_targets(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
) -> Vec<ObjectId> {
    if let Pending::Priority { player, .. } = engine.pending().clone()
        && player != caster
    {
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    cast_from_hand(engine, caster, false_orders());
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player, options, ..
        } => {
            assert_eq!(player, caster);
            sorted(options)
        }
        other => panic!("expected the target question, got {other:?}"),
    }
}

/// Answers the target question with `target` and lets the spell resolve up
/// to its re-block question, whose menu is returned (empty when none was
/// asked, which every caller of this tells apart from a menu it expects).
#[track_caller]
fn resolve_at(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    target: ObjectId,
) -> Vec<ObjectId> {
    engine
        .apply(
            caster,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::BlockWith { blocker },
                ..
            } => {
                assert_eq!((player, blocker), (caster, target), "the caster names it");
                return sorted(options);
            }
            Pending::Priority { player, .. } if !stack_is_empty(engine) => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { .. } => return Vec::new(),
            other => panic!("while the spell resolved: {other:?}"),
        }
    }
    panic!("the spell never resolved: {:?}", engine.pending());
}

/// Casts at `target` and returns the re-block menu.
#[track_caller]
fn false_orders_at(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    target: ObjectId,
) -> Vec<ObjectId> {
    let offered = cast_and_offered_targets(engine, caster);
    assert!(offered.contains(&target), "{target:?} is not offered");
    resolve_at(engine, caster, target)
}

/// "Block it": the caster answers the re-block question with `attacker`.
#[track_caller]
fn block_with_it(engine: &mut Engine<RegistryLookup>, caster: PlayerId, attacker: ObjectId) {
    engine
        .apply(
            caster,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
}

/// "No": the caster answers the re-block question with nothing.
#[track_caller]
fn decline_the_block(engine: &mut Engine<RegistryLookup>, caster: PlayerId) {
    engine
        .apply(caster, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the question is answered: {:?}",
        engine.pending()
    );
}

#[track_caller]
fn to_second_main(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
}

fn became_blocker_since(
    engine: &Engine<RegistryLookup>,
    since: u64,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|e| e.seq > since)
        .any(|e| {
            matches!(e.event, GameEvent::BecameBlocker { object, attacker: a }
                if object == blocker && a == attacker)
        })
}

/// "Cast this spell only during the declare blockers step": not in the first
/// main phase, not in the declare attackers step, but in the declare blockers
/// step of either player's turn, by either player.
#[test]
fn false_orders_is_castable_only_in_the_declare_blockers_step() {
    // The first main phase of the caster's own turn.
    let mut engine = mirrored(7001);
    reach_main_phase(&mut engine, P0);
    assert!(!castable_now(&mut engine, P0), "not in main phase 1");

    // The declare attackers step, with a creature of the defender's to
    // target, so that nothing but the step can be what refuses it.
    let mut engine = mirrored(7002);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    declare_attackers(&mut engine, P0, &[(giant, Defender::Player(P1))]);
    assert!(!castable_now(&mut engine, P0), "not in declare attackers");

    // The declare blockers step of P0's turn: P0, the attacker, may cast it.
    let mut engine = mirrored(7003);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    to_blocks(&mut engine, P0, &[(giant, Defender::Player(P1))], &[]);
    assert!(castable_now(&mut engine, P0), "in P0's declare blockers");

    // The declare blockers step of P1's turn: P1, the attacker, may cast it,
    // at a creature P0 controls.
    let mut engine = mirrored(7004);
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    to_blocks(&mut engine, P1, &[(giant, Defender::Player(P0))], &[]);
    assert!(castable_now(&mut engine, P1), "in P1's declare blockers");

    // ... and P0, the defender, once P1 passes priority: the spell is not
    // the active player's alone.
    let mut engine = mirrored(7005);
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    to_blocks(&mut engine, P1, &[(giant, Defender::Player(P0))], &[]);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(
        castable_now(&mut engine, P0),
        "the defender may cast it too"
    );
}

/// "Target creature defending player controls": the defender's creatures, a
/// blocker or not, and no attacker, no creature of the attacker's, no
/// permanent that is not a creature. A target outside that is refused and the
/// question stays open.
#[test]
fn the_target_is_a_creature_the_defending_player_controls() {
    let mut engine = Duel::new(7010, mountain())
        .battlefield(0, &[mountain(), hill_giant(), grizzly_bears()])
        .battlefield(1, &[mountain(), serra_angel(), grizzly_bears()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let own_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let own_mountain = on_battlefield(&engine, P0, mountain()).unwrap();
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let their_mountain = on_battlefield(&engine, P1, mountain()).unwrap();
    to_blocks(
        &mut engine,
        P0,
        &[(giant, Defender::Player(P1))],
        &[(angel, giant)],
    );

    let offered = cast_and_offered_targets(&mut engine, P0);
    assert_eq!(
        offered,
        sorted(vec![angel, their_bears]),
        "the defender's creatures: the one that blocks and the one that does not"
    );
    for refused in [giant, own_bears, own_mountain, their_mountain] {
        assert!(
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![refused],
                        players: vec![],
                    },
                )
                .is_err(),
            "{refused:?} is not a creature the defending player controls"
        );
        assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    }
    // The one that does not block is a legal target too: it leaves combat
    // from nothing, and the spell asks about the re-block all the same.
    let menu = resolve_at(&mut engine, P0, their_bears);
    assert_eq!(menu, vec![giant]);
    assert!(
        engine.state().combat.is_blocked(giant),
        "the Angel still blocks the Giant"
    );
}

/// The Angel alone blocks the Giant. False Orders takes it out of combat, the
/// Giant becomes unblocked, and the caster declines the re-block: the Giant
/// deals its damage to the player, and the Angel is out of combat, so the
/// Giant takes nothing.
#[test]
fn an_attacker_only_the_target_blocked_deals_its_damage_to_the_player() {
    let mut engine = mirrored(7020);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    to_blocks(
        &mut engine,
        P0,
        &[(giant, Defender::Player(P1))],
        &[(angel, giant)],
    );
    assert!(engine.state().combat.is_blocked(giant));

    assert_eq!(false_orders_at(&mut engine, P0, angel), vec![giant]);
    assert!(!engine.state().combat.is_blocked(giant), "unblocked");
    assert!(engine.state().combat.is_attacking(giant), "still attacking");
    decline_the_block(&mut engine, P0);

    let before = life(&engine, P1);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, P1), before - 3, "the Giant hits the player");
    assert!(
        on_battlefield(&engine, P0, hill_giant()).is_some(),
        "the Angel, out of combat, dealt no damage"
    );
    assert!(on_battlefield(&engine, P1, serra_angel()).is_some());
}

/// The Angel and a Bears both block the Giant. Removing the Angel leaves the
/// Giant blocked (CR 509.1h: the Bears blocked it too): no damage to the
/// player, all of it to the Bears, none to or from the Angel.
#[test]
fn an_attacker_a_second_creature_also_blocked_stays_blocked() {
    let mut engine = Duel::new(7030, mountain())
        .battlefield(0, &[mountain(), hill_giant()])
        .battlefield(1, &[serra_angel(), grizzly_bears()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    to_blocks(
        &mut engine,
        P0,
        &[(giant, Defender::Player(P1))],
        &[(angel, giant), (bears, giant)],
    );

    assert_eq!(false_orders_at(&mut engine, P0, angel), vec![giant]);
    assert!(engine.state().combat.is_blocked(giant), "still blocked");
    assert!(engine.state().combat.blocked_by(angel).is_empty());
    assert!(engine.state().combat.is_blocking(bears, giant));
    decline_the_block(&mut engine, P0);

    let before = life(&engine, P1);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, P1), before, "blocked, so no damage to p1");
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, P0, hill_giant()).is_some(),
        "the Bears dealt 2 to a 3/3, and the Angel, out of combat, nothing"
    );
    assert!(on_battlefield(&engine, P1, serra_angel()).is_some());
}

/// The caster has the Leprechaun, which was not blocking, block the Giant: a
/// block, journalled, and the Leprechaun's "whenever this creature blocks"
/// trigger goes on the stack and turns the red Giant green. A block that
/// never happened could not have done that.
#[test]
fn the_re_block_is_a_block_and_a_block_trigger_fires() {
    let mut engine = Duel::new(7040, mountain())
        .battlefield(0, &[mountain(), hill_giant()])
        .battlefield(1, &[aisling_leprechaun()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let leprechaun = on_battlefield(&engine, P1, aisling_leprechaun()).unwrap();
    to_blocks(&mut engine, P0, &[(giant, Defender::Player(P1))], &[]);
    assert_eq!(colors_of(&engine, giant), red());
    assert!(!engine.state().combat.is_blocked(giant));

    assert_eq!(false_orders_at(&mut engine, P0, leprechaun), vec![giant]);
    let since = engine.state().journal.last_seq();
    block_with_it(&mut engine, P0, giant);
    assert!(engine.state().combat.is_blocking(leprechaun, giant));
    assert!(
        became_blocker_since(&engine, since, leprechaun, giant),
        "the new block is journalled"
    );
    assert!(
        !stack_is_empty(&engine),
        "the block trigger is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        colors_of(&engine, giant),
        green(),
        "the Leprechaun blocked it, and its trigger ran"
    );

    let before = life(&engine, P1);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, P1), before, "blocked, so no damage to p1");
}

/// The control of the test before it: the same Leprechaun, the same spell,
/// the question declined. No block was made, so no block trigger fires, and
/// the Giant stays red and unblocked.
#[test]
fn declining_the_re_block_fires_no_block_trigger() {
    let mut engine = Duel::new(7041, mountain())
        .battlefield(0, &[mountain(), hill_giant()])
        .battlefield(1, &[aisling_leprechaun()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let leprechaun = on_battlefield(&engine, P1, aisling_leprechaun()).unwrap();
    to_blocks(&mut engine, P0, &[(giant, Defender::Player(P1))], &[]);

    let since = engine.state().journal.last_seq();
    assert_eq!(false_orders_at(&mut engine, P0, leprechaun), vec![giant]);
    decline_the_block(&mut engine, P0);
    assert!(stack_is_empty(&engine), "no trigger");
    assert!(!became_blocker_since(&engine, since, leprechaun, giant));
    assert!(engine.state().combat.blocked_by(leprechaun).is_empty());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(colors_of(&engine, giant), red());
    assert!(!engine.state().combat.is_blocked(giant));
}

/// Declined, the target stays out of combat: the two Giants would trade, but
/// one is no longer blocking, so it deals no combat damage and takes none.
#[test]
fn declining_the_re_block_leaves_the_creature_out_of_combat() {
    let mut engine = Duel::new(7050, mountain())
        .battlefield(0, &[mountain(), hill_giant()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let attacker = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let blocker = on_battlefield(&engine, P1, hill_giant()).unwrap();
    to_blocks(
        &mut engine,
        P0,
        &[(attacker, Defender::Player(P1))],
        &[(blocker, attacker)],
    );

    assert_eq!(false_orders_at(&mut engine, P0, blocker), vec![attacker]);
    decline_the_block(&mut engine, P0);
    assert!(engine.state().combat.blocked_by(blocker).is_empty());
    assert!(
        !engine
            .state()
            .combat
            .blockers
            .iter()
            .any(|b| b.blocker == blocker),
        "it is not a blocker any more"
    );
    assert!(!engine.state().combat.is_blocked(attacker));

    let before = life(&engine, P1);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, P1), before - 3, "the unblocked Giant hits");
    assert!(
        on_battlefield(&engine, P0, hill_giant()).is_some(),
        "the removed Giant dealt it no damage"
    );
    assert!(
        on_battlefield(&engine, P1, hill_giant()).is_some(),
        "and took none from it"
    );
}

/// An attacker that was not blocked at all, and is blocked by the re-block:
/// it stays out of the player's life total and meets the Angel instead.
#[test]
fn re_blocking_an_unblocked_attacker_makes_it_blocked() {
    let mut engine = mirrored(7060);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    to_blocks(&mut engine, P0, &[(giant, Defender::Player(P1))], &[]);
    assert!(!engine.state().combat.is_blocked(giant), "unblocked");

    assert_eq!(false_orders_at(&mut engine, P0, angel), vec![giant]);
    block_with_it(&mut engine, P0, giant);
    assert!(engine.state().combat.is_blocked(giant), "blocked");
    assert!(engine.state().combat.is_blocking(angel, giant));

    let before = life(&engine, P1);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, P1), before, "blocked, so no damage to p1");
    assert!(
        in_graveyard(&engine, P0, hill_giant()).is_some(),
        "and the Angel killed it"
    );
    assert!(on_battlefield(&engine, P1, serra_angel()).is_some());
}

/// The same from the other side of the table: P1 attacks, P0 defends and
/// casts it on the creature P0 controls, and P0 has it block P1's attacker.
#[test]
fn the_defender_may_cast_it_on_their_own_creature_and_have_it_block() {
    let mut engine = mirrored(7061);
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, P0, serra_angel()).unwrap();
    to_blocks(&mut engine, P1, &[(giant, Defender::Player(P0))], &[]);
    let offered = cast_and_offered_targets(&mut engine, P0);
    assert_eq!(
        offered,
        sorted(vec![
            angel,
            on_battlefield(&engine, P0, hill_giant()).unwrap()
        ]),
        "P0's own creatures"
    );
    let menu = resolve_at(&mut engine, P0, angel);
    assert_eq!(menu, vec![giant], "P1's attacker, and only P0 names it");
    block_with_it(&mut engine, P0, giant);
    assert!(engine.state().combat.is_blocking(angel, giant));
}

/// A three-seat table: P0 attacks P1 with two Giants and P2 with a third. P1's
/// Angel is the target, and only the attackers of P1's are offered; P2's
/// Bears as the target offers only the one that attacks P2.
#[test]
fn only_attackers_of_the_targets_controller_are_offered() {
    let build = |seed: u64| {
        let mut engine = Duel::table(seed, mountain(), 3)
            .battlefield(0, &[mountain(), hill_giant(), hill_giant(), hill_giant()])
            .battlefield(1, &[serra_angel()])
            .battlefield(2, &[grizzly_bears()])
            .hand(0, &[false_orders()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let giants = all_on_battlefield(&engine, P0, hill_giant());
        assert_eq!(giants.len(), 3);
        let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
        let bears = on_battlefield(&engine, P2, grizzly_bears()).unwrap();
        to_blocks(
            &mut engine,
            P0,
            &[
                (giants[0], Defender::Player(P1)),
                (giants[1], Defender::Player(P1)),
                (giants[2], Defender::Player(P2)),
            ],
            &[(angel, giants[0]), (bears, giants[2])],
        );
        (engine, giants, angel, bears)
    };

    let (mut engine, giants, angel, bears) = build(7070);
    let offered = cast_and_offered_targets(&mut engine, P0);
    assert_eq!(
        offered,
        sorted(vec![angel, bears]),
        "both attacked players' creatures"
    );
    assert_eq!(
        resolve_at(&mut engine, P0, angel),
        sorted(vec![giants[0], giants[1]]),
        "P1's attackers, not the one attacking P2"
    );
    block_with_it(&mut engine, P0, giants[1]);
    assert!(engine.state().combat.is_blocking(angel, giants[1]));
    assert!(!engine.state().combat.is_blocking(angel, giants[2]));

    let (mut engine, giants, _, bears) = build(7071);
    assert_eq!(
        false_orders_at(&mut engine, P0, bears),
        vec![giants[2]],
        "P2's creature: only the attacker of P2"
    );
}

/// "Defending player" at a table of three is a player an attacker is
/// attacking. P2 is attacked by nothing here, so their creature is not a
/// target.
#[test]
#[ignore = "Filter::ControlledByDefendingPlayer (eval.rs) accepts any opponent of the active player during combat, so the creature of a player nobody attacks is offered as a target (CR 506.2: the defending player is the player a creature is attacking)"]
fn a_creature_of_a_player_nobody_attacks_is_not_a_target() {
    let mut engine = Duel::table(7072, mountain(), 3)
        .battlefield(0, &[mountain(), hill_giant()])
        .battlefield(1, &[serra_angel()])
        .battlefield(2, &[grizzly_bears()])
        .hand(0, &[false_orders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, P1, serra_angel()).unwrap();
    to_blocks(
        &mut engine,
        P0,
        &[(giant, Defender::Player(P1))],
        &[(angel, giant)],
    );
    let offered = cast_and_offered_targets(&mut engine, P0);
    assert_eq!(offered, vec![angel], "P1 is the only defending player");
}
