//! Protection with real cards: Death Speakers ({W}, 1/1, protection from
//! black) against Fell ({1}{B}) and Hired Poisoner ({B}, 1/1 deathtouch).
//! Protection stops targeting, blocking by, and damage from what it names
//! (CR 702.16b, e, f) — and nothing else: Pyroclasm, which is red, still
//! deals its damage.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender, ObjectId};

const SEED: u64 = 0x9a07_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn speakers() -> CardIndex {
    card_index("4a1db129-cc7c-410a-a41e-d633e30cfdad")
}

fn poisoner() -> CardIndex {
    card_index("a7a6cdfd-10ac-49d0-a0a6-8ad37e326511")
}

fn fell() -> CardIndex {
    card_index("8f968470-ce00-43a3-95f5-2e3fe987be19")
}

fn pyroclasm() -> CardIndex {
    card_index("e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn main_phase(engine: &mut Game) {
    pass_until(engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
}

fn offered_by(engine: &mut Game, spell: CardIndex) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, spell).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_black_spell_cannot_target_it() {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[fell()])
        .battlefield(0, &[swamp(), swamp(), poisoner()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    main_phase(&mut engine);
    let cleric = on_battlefield(&engine, P1, speakers()).unwrap();
    let rogue = on_battlefield(&engine, P0, poisoner()).unwrap();
    let options = offered_by(&mut engine, fell());
    assert!(!options.contains(&cleric));
    assert!(
        options.contains(&rogue),
        "an unprotected creature is offered"
    );
}

#[test]
fn a_red_sweeper_still_damages_it() {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[pyroclasm()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    main_phase(&mut engine);
    super::testkit::cast_from_hand(&mut engine, P0, pyroclasm());
    pass_until(&mut engine, stack_is_empty);
    assert!(super::testkit::in_graveyard(&engine, P1, speakers()).is_some());
    assert!(on_battlefield(&engine, P1, speakers()).is_none());
}

/// Seat 0 attacks seat 1 (who has a black Hired Poisoner) with `attacker`,
/// and the game is stopped at the point where blocks are declared.
fn attack_into_poisoner(attacker_card: CardIndex) -> (Game, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[attacker_card])
        .battlefield(1, &[poisoner()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let attacker = on_battlefield(&engine, P0, attacker_card).unwrap();
    let rogue = on_battlefield(&engine, P1, poisoner()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    (engine, attacker, rogue)
}

/// The black Poisoner cannot block the protected cleric (CR 702.16f): the
/// block is not a legal pair and the cleric connects.
#[test]
fn a_black_creature_cannot_block_it() {
    let (mut engine, cleric, rogue) = attack_into_poisoner(speakers());
    assert!(!crate::combat::can_block(engine.state(), P1, rogue, cleric));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, P1, poisoner()).is_some());
}

/// The control: the same Poisoner may block an unprotected creature, and
/// the engine asks about it.
#[test]
fn a_black_creature_can_block_an_unprotected_one() {
    let (mut engine, elf, rogue) = attack_into_poisoner(super::testkit::quiet_creature());
    assert!(crate::combat::can_block(engine.state(), P1, rogue, elf));
    assert!(matches!(engine.pending(), Pending::ChooseBlockers { .. }));
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(rogue, elf)],
            },
        )
        .unwrap();
}

#[test]
fn damage_from_a_black_deathtoucher_is_prevented() {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[poisoner()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let rogue = on_battlefield(&engine, P0, poisoner()).unwrap();
    let cleric = on_battlefield(&engine, P1, speakers()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(cleric, rogue)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, speakers()).is_some(),
        "no damage, so no deathtouch"
    );
    assert!(
        on_battlefield(&engine, P0, poisoner()).is_none(),
        "the cleric's damage is not prevented"
    );
}

fn paladin_en_vec() -> CardIndex {
    card_index("fd8722e1-9c41-4037-ab7e-47e2aa9858a0")
}

fn disciple_of_grace() -> CardIndex {
    card_index("7f4ceca9-0f55-4fa6-8e64-befbc4303d4e")
}

fn plague_spitter() -> CardIndex {
    card_index("5feedfb0-30e6-400d-9e28-d541ea1aa14e")
}

/// Seat 0's Paladin en-Vec and Disciple of Grace, both with protection from
/// black, under seat 1's Plague Spitter, which deals 1 damage to each
/// creature and each player at seat 1's upkeep.
fn paladin_and_disciple() -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[paladin_en_vec(), disciple_of_grace()])
        .battlefield(1, &[plague_spitter()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn seat_one_main(engine: &Game) -> bool {
    engine.state().turn.active == P1 && engine.state().turn.phase == Phase::FirstMain
}

/// Protection on two different creatures is two effects on two objects,
/// and nobody chooses an order between them: CR 616.1 asks the affected
/// object's controller only when two or more effects try to modify how the
/// event affects that one object. Each protection prevents its own
/// creature's damage whichever applies first.
///
/// The record: Carbon's ladder game hc-expert-b/game-0011, turn 13. Plague
/// Spitter's upkeep damage to each creature asked seat 0 which of Paladin
/// en-Vec's and Disciple of Grace's protection from black applied next.
///
/// The question was the whole difference: the engine as it was, answered
/// either way, reaches the same board, which hashes as this one does.
#[test]
fn protection_on_two_creatures_asks_no_order() {
    let mut engine = paladin_and_disciple();
    let paladin = on_battlefield(&engine, P0, paladin_en_vec()).unwrap();
    let disciple = on_battlefield(&engine, P0, disciple_of_grace()).unwrap();
    let spitter = on_battlefield(&engine, P1, plague_spitter()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. }) || seat_one_main(e)
    });
    assert!(
        !matches!(engine.pending(), Pending::ChooseDamageEffect { .. }),
        "asked an order between two creatures' protection: {:?}",
        engine.pending()
    );
    for first in [0, 1] {
        let then = crate::damage::ASK_EVERY_ORDER.with(|ask| {
            ask.set(true);
            let mut then = paladin_and_disciple();
            pass_until(&mut then, |e| {
                matches!(e.pending(), Pending::ChooseDamageEffect { .. })
            });
            let Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } = then.pending().clone()
            else {
                unreachable!()
            };
            assert_eq!(options.len(), 2, "the control: as it was, it asked");
            then.apply(
                player,
                PlayerAction::ChooseDamageEffect {
                    choice,
                    effect: options[first].id,
                },
            )
            .unwrap();
            pass_until(&mut then, seat_one_main);
            ask.set(false);
            then
        });
        assert_eq!(then.snapshot_hash(), engine.snapshot_hash(), "{first}");
    }
    let damage = |id| engine.state().object(id).unwrap().damage;
    assert_eq!(damage(paladin), 0, "the Paladin's protection prevented it");
    assert_eq!(
        damage(disciple),
        0,
        "the Disciple's protection prevented it"
    );
    assert_eq!(damage(spitter), 1, "the Spitter damaged itself");
    for seat in [P0, P1] {
        assert_eq!(
            engine.state().players[usize::from(seat.get())].life,
            19,
            "each player took 1"
        );
    }
}

fn pestilence() -> CardIndex {
    card_index("dafe63ef-f3d6-45e7-877a-573da92ba85e")
}

fn pariah() -> CardIndex {
    card_index("2c5c8250-1860-42a1-a335-071f54830d37")
}

/// Seat 0's Pestilence and Swamp beside its Paladin en-Vec, which wears seat
/// 0's Pariah, in seat 0's first main phase: `(engine, paladin)`.
fn pariah_on_the_paladin() -> (Game, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[pestilence(), swamp(), paladin_en_vec(), pariah()])
        .start();
    let paladin = on_battlefield(&engine, P0, paladin_en_vec()).unwrap();
    let aura = on_battlefield(&engine, P0, pariah()).unwrap();
    {
        // What a starting battlefield cannot say, set before the first
        // state-based check would put the Aura into a graveyard.
        let state = engine.dev_state_mut(P0).expect("the harness trusts itself");
        state.object_mut(aura).expect("seated").attached_to = Some(paladin);
        state.invalidate_projections();
    }
    keep_mulligans(&mut engine);
    main_phase(&mut engine);
    (engine, paladin)
}

/// Activates Pestilence ({B}: 1 damage to each creature and each player) and
/// passes until it has resolved or a damage effect is to be chosen.
fn pestilence_once(engine: &mut Game) {
    let source = on_battlefield(engine, P0, pestilence()).unwrap();
    tap_all_mana(engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
            || (stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0))
    });
}

/// A redirection beside protection, where every order prevents everything.
/// Pestilence deals 1 to the Paladin and 1 to seat 0: protection from
/// black prevents the first (CR 702.16e), and Pariah sends the second to the
/// Paladin, whose protection prevents it there too. CR 616.1 has seat 0
/// choose among effects on one object or player; the Paladin's damage meets
/// only the protection and seat 0's only Pariah, and the protection is not
/// spent, so the redirected damage meets it as well whichever came first.
///
/// The record: Carbon's ladder game hc-expert-d2/game-0010, q179, turn 14,
/// which asked seat 0 to order the Paladin's protection and Pariah.
#[test]
fn a_redirection_onto_protection_asks_no_order() {
    let (mut engine, paladin) = pariah_on_the_paladin();
    pestilence_once(&mut engine);
    assert!(
        !matches!(engine.pending(), Pending::ChooseDamageEffect { .. }),
        "asked an order between protection and Pariah: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().object(paladin).unwrap().damage,
        0,
        "both points were sent to the Paladin and prevented there"
    );
    assert_eq!(engine.state().players[0].life, 20, "Pariah took seat 0's");
    assert_eq!(engine.state().players[1].life, 19, "seat 1 took 1");
    for first in [0, 1] {
        let then = crate::damage::ASK_EVERY_ORDER.with(|ask| {
            ask.set(true);
            let (mut then, _) = pariah_on_the_paladin();
            pestilence_once(&mut then);
            let Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } = then.pending().clone()
            else {
                panic!("the control: as it was, it asked: {:?}", then.pending())
            };
            assert_eq!(options.len(), 2, "protection and Pariah");
            then.apply(
                player,
                PlayerAction::ChooseDamageEffect {
                    choice,
                    effect: options[first].id,
                },
            )
            .unwrap();
            pestilence_answered(&mut then);
            ask.set(false);
            then
        });
        assert_eq!(then.snapshot_hash(), engine.snapshot_hash(), "{first}");
    }
}

/// Passes until Pestilence has resolved, choosing the first offered damage
/// effect on the way.
fn pestilence_answered(engine: &mut Game) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => engine
                .apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: options[0].id,
                    },
                )
                .unwrap(),
            Pending::Priority { player, .. } if stack_is_empty(engine) && player == P0 => {
                return;
            }
            _ => pass_until(engine, |e| {
                matches!(e.pending(), Pending::ChooseDamageEffect { .. })
                    || (stack_is_empty(e)
                        && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0))
            }),
        }
    }
    panic!("Pestilence never finished: {:?}", engine.pending());
}
