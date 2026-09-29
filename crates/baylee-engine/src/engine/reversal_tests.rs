//! A reversed action gives back what was paid for it (CR 732.1).
//!
//! "If a player takes an illegal action or starts to take an action but
//! can't legally complete it, the entire action is reversed and any payments
//! already made are canceled." A cast and an activation are paid part by
//! part, the mana first and all at once, and a part after it could refuse:
//! the cast was then reversed and the activation handed priority back with
//! the mana spent and the costs paid by then left paid.
//!
//! No card in the pool reaches that today by itself. Every question is asked
//! before anything is paid, and every answer is checked against the list the
//! question offered, so the parts that can refuse after the mana (a move of
//! an object that is gone, a counter that is not there) refuse only for an
//! object that stopped existing between the question and its answer. The
//! tests here make it stop existing, with the harness' own capability, to
//! reach the parts the payment has and the pool does not yet use.

#[allow(clippy::wildcard_imports)] // the shared duel plumbing, as every card test takes it
use super::testkit::*;

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// "{2}, {T}, Sacrifice a creature: Draw a card."
fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

/// "As an additional cost to cast this spell, sacrifice a green creature."
fn natural_order() -> CardIndex {
    card_index("8c1fe337-375a-4add-93b6-0ac39ed72b4f")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// "When this creature enters, exile up to one other target non-Fox
/// creature until this creature leaves the battlefield."
fn werefox_bodyguard() -> CardIndex {
    card_index("d5ee2ced-29f4-430f-962e-2f930b92624c")
}

/// Takes `object` out of `from` and out of the game altogether, as a token
/// that left the battlefield does (CR 111.7): whatever still names it names
/// nothing. `from` is the zone the question asked about it in — the
/// battlefield for a permanent a cost points at, the caster's hand for a
/// pitch card — so a card gone from hand is gone the same way a card gone
/// from the battlefield is.
#[track_caller]
fn cease(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    object: ObjectId,
    from: ZoneLocation,
) {
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up");
    assert!(state.zones.remove(object, from));
    assert!(state.arena.remove(object).is_some());
}

fn pool(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().players[seat.get() as usize]
        .mana_pool
        .total()
}

/// Phyrexian Vault answered with a creature that is gone: the activation is
/// reversed with its {2} back in the pool and the Vault untapped.
#[test]
fn a_reversed_activation_gives_its_mana_and_its_tap_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(
            0,
            &[
                phyrexian_vault(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vault = on_battlefield(&engine, p0, phyrexian_vault()).expect("the Vault");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(pool(&engine, p0), 2);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "{:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: vault,
                ability_index: 0,
            },
        )
        .expect("the Vault activates");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "the two Elves: {options:?}");
    let gone = options[0];
    cease(&mut engine, p0, gone, ZoneLocation::Battlefield);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gone],
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activation is reversed and priority is the activator's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was activated");
    assert_eq!(pool(&engine, p0), 2, "the {{2}} is back in the pool");
    assert!(
        !engine
            .state()
            .object(vault)
            .expect("the Vault")
            .status
            .contains(crate::object::Status::TAPPED),
        "the {{T}} is paid back"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the other Elves stand"
    );
}

/// Natural Order answered with a green creature that is gone: the cast is
/// reversed with its {2}{G}{G} back in the pool and the card back in hand.
#[test]
fn a_reversed_cast_gives_its_mana_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[natural_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| *id != elves)
        .collect();
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(pool(&engine, p0), 5);
    cast_with_floating(&mut engine, p0, natural_order());
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![elves]);
    cease(&mut engine, p0, elves, ZoneLocation::Battlefield);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the cast is reversed and priority is the caster's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was cast");
    assert!(
        in_hand(&engine, p0, natural_order()).is_some(),
        "the spell is back where it came from"
    );
    assert_eq!(
        pool(&engine, p0),
        5,
        "the {{2}}{{G}}{{G}} is back in the pool"
    );
}

/// A reversed sacrifice puts back what its leaving set free.
///
/// Werefox Bodyguard holds seat 1's Elves in exile until it leaves the
/// battlefield, and they return inside the move that takes it away (CR
/// 610.3, `GameState::move_object`), so a cost that sacrifices it sets them
/// free while the payment is still running. When a later part of that cost
/// refuses, the whole action is reversed (CR 732.1): the Bodyguard stands,
/// the Elves are in exile again as the object they were and held by it, and
/// the journal has neither move. No card prints a sacrifice followed by a
/// part that can refuse; this cost is written for the test, and asks for a
/// counter the Bodyguard does not have.
#[test]
fn a_reversed_sacrifice_puts_back_what_its_leaving_set_free() {
    static SACRIFICE_THEN_A_COUNTER: [CostPart; 2] = [
        CostPart::SacrificeSelf,
        CostPart::RemoveCounterSelf {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            n: 1,
        },
    ];
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(614, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[werefox_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("seat 1's Elves");
    cast_from_hand(&mut engine, p0, werefox_bodyguard());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are a target");
    pass_until(&mut engine, stack_is_empty);
    let bodyguard = on_battlefield(&engine, p0, werefox_bodyguard()).expect("the Bodyguard");
    let held = |engine: &Engine<RegistryLookup>| {
        engine.state().object(elves).is_some_and(|o| {
            o.zone == crate::zone::Zone::Exile
                && o.riders.iter().any(
                    |r| matches!(r, crate::object::Rider::Linked { host, .. } if *host == bodyguard),
                )
        })
    };
    assert!(held(&engine), "the Bodyguard holds the Elves");
    let version = engine.state().object(elves).map(|o| o.version);
    let journal = engine.state().journal.len();

    let cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &SACRIFICE_THEN_A_COUNTER,
    };
    assert!(
        engine.pay_cost(p0, bodyguard, &cost, &[], 0).is_err(),
        "the Bodyguard has no counter to remove"
    );

    assert_eq!(
        engine.state().object(bodyguard).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the sacrifice is reversed"
    );
    assert!(held(&engine), "and the Bodyguard holds the Elves again");
    assert_eq!(
        engine.state().object(elves).map(|o| o.version),
        version,
        "the object they were"
    );
    assert_eq!(
        engine.state().journal.len(),
        journal,
        "neither move is in the journal"
    );
}

/// "Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard."
fn uro_titan_of_nature_s_wrath() -> CardIndex {
    card_index("ee302659-59ed-4eef-babe-451b9ccf7f14")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

/// Uro escaped with five other cards, the fifth of which is gone by the
/// time the answer is paid: four were exiled and the mana was spent before
/// the fifth refused. The cast is reversed (CR 732.1), and the four are back
/// in the graveyard as they were, Uro with them, and the {G}{G}{U}{U} back
/// in the pool. The gone card is named last on purpose, so the refusal
/// comes after exiles that a reversal without the roll-back would keep.
#[test]
fn a_reversed_escape_puts_the_exiled_cards_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[uro_titan_of_nature_s_wrath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let uro = in_hand(&engine, p0, uro_titan_of_nature_s_wrath()).expect("Uro in hand");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                uro,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    seed_graveyard(&mut engine, p0, 5);
    tap_all_mana(&mut engine, p0);
    assert_eq!(pool(&engine, p0), 4);
    engine
        .apply(p0, PlayerAction::CastSpell { card: uro })
        .expect("Uro escapes");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let escape = options
            .iter()
            .position(|o| o.kind == crate::choice::CastModeKind::Escape)
            .expect("escape is offered");
        engine
            .apply(p0, PlayerAction::ChooseMode(escape))
            .expect("escape is chosen");
    }
    let Pending::ChooseCards {
        options,
        prompt: crate::choice::ChoicePrompt::CostExile,
        ..
    } = engine.pending().clone()
    else {
        panic!("escape asks which five, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "the five other cards: {options:?}");
    let gone = options[4];
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        assert!(state.zones.remove(gone, ZoneLocation::Graveyard(p0)));
        assert!(state.arena.remove(gone).is_some());
    }

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options.clone(),
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the cast is reversed and priority is the caster's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was cast");
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0));
    assert!(graveyard.contains(&uro), "Uro is back where it came from");
    for card in &options[..4] {
        assert!(
            graveyard.contains(card),
            "{card:?} was exiled for the cost and is back"
        );
    }
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .is_empty(),
        "nothing stays exiled"
    );
    assert_eq!(
        pool(&engine, p0),
        4,
        "the {{G}}{{G}}{{U}}{{U}} is back in the pool"
    );
}

/// "You may pay 1 life and exile a blue card from your hand rather than
/// pay this spell's mana cost." / "Counter target spell."
fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}

/// "Counter target spell." — a blue card, the only one Force of Will finds
/// to pitch on the board these tests build.
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}

/// Force of Will pitched (CR 118.9): a blue card exiled from hand and 1
/// life pay for it rather than its mana cost. The pitched card is gone by
/// the time the answer is paid — the harness makes it so, the way it makes
/// the Vault's creature and Uro's fifth card so above. The cast is
/// reversed (CR 732.1): the life is back, Force of Will is back in hand,
/// and the spell it would have countered sits on the stack exactly as it
/// did before the question was asked.
#[test]
fn a_reversed_pitch_gives_the_life_back_and_the_card_stays_in_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(701, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[force_of_will(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    let elves_spell = on_stack(&engine, llanowar_elves()).expect("the Elves spell");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p0 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("p0 passes after casting");

    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let fow = in_hand(&engine, p1, force_of_will()).expect("Force of Will in hand");
    assert!(
        legal.castable.contains(&fow),
        "Force of Will must be castable via pitch: {:?}",
        legal.castable
    );
    let life_before = engine.state().players[p1.get() as usize].life;
    let journal = engine.state().journal.len();

    engine
        .apply(p1, PlayerAction::CastSpell { card: fow })
        .expect("Force of Will begins casting");

    // With an empty pool only the pitch alternative is payable, so the
    // wizard auto-selects it and asks for targets directly.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![elves_spell]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elves_spell],
                players: vec![],
            },
        )
        .expect("the Elves spell is a legal target");

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected the pitch choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert_eq!(options.len(), 1, "only Counterspell is blue: {options:?}");
    let pitch_card = options[0];
    cease(&mut engine, p1, pitch_card, ZoneLocation::Hand(p1));

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch_card],
            },
        )
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "the cast is reversed and priority is the caster's: {:?}",
        engine.pending()
    );
    assert!(
        in_hand(&engine, p1, force_of_will()).is_some(),
        "Force of Will is back in hand"
    );
    assert_eq!(
        engine.state().players[p1.get() as usize].life,
        life_before,
        "no life is paid"
    );
    assert_eq!(pool(&engine, p1), 0, "no mana is spent");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .is_empty(),
        "nothing is exiled"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).to_vec(),
        vec![elves_spell],
        "the spell Force of Will would have countered is untouched"
    );
    assert_eq!(
        engine.state().journal.len(),
        journal,
        "nothing is journaled"
    );
}

/// "{T}: Exile up to two target cards from a single graveyard." /
/// "Unlicensed Hearse's power and toughness are each equal to the number
/// of cards exiled with it."
fn unlicensed_hearse() -> CardIndex {
    card_index("c640654c-487e-4a2c-aced-126ed835b78f")
}

/// Unlicensed Hearse's graveyard choice (CR 601.2c, ahead of CR 601.2h's
/// payment): with more than one graveyard holding a card, the activation
/// asks which one before it asks which cards. The Hearse itself is gone by
/// the time the answer is paid — the harness makes it so, the way the
/// other tests here make a creature or a graveyard card so. The
/// activation is reversed (CR 732.1, 732.2): neither graveyard is
/// touched, nothing is exiled, and priority returns to the activator.
#[test]
fn a_reversed_graveyard_choice_touches_neither_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(702, forest())
        .battlefield(0, &[unlicensed_hearse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let hearse = on_battlefield(&engine, p0, unlicensed_hearse()).expect("the Hearse");
    let p0_graveyard_before = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .to_vec();
    let p1_graveyard_before = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .to_vec();
    let life0_before = engine.state().players[p0.get() as usize].life;
    let life1_before = engine.state().players[p1.get() as usize].life;
    let journal = engine.state().journal.len();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hearse, 0)),
        "{:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hearse,
                ability_index: 0,
            },
        )
        .expect("the Hearse activates");

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("expected the graveyard choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 2, "both graveyards hold a card: {options:?}");
    let chosen = options[0];
    cease(&mut engine, p0, hearse, ZoneLocation::Battlefield);

    engine
        .apply(p0, PlayerAction::ChoosePlayer(chosen))
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activation is reversed and priority is the activator's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was activated");
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .to_vec(),
        p0_graveyard_before,
        "p0's graveyard is untouched"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .to_vec(),
        p1_graveyard_before,
        "p1's graveyard is untouched"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .is_empty()
            && engine
                .state()
                .zones
                .list(ZoneLocation::Exile(p1))
                .is_empty(),
        "nothing is exiled"
    );
    assert_eq!(
        engine.state().players[p0.get() as usize].life,
        life0_before,
        "p0's life is unchanged"
    );
    assert_eq!(
        engine.state().players[p1.get() as usize].life,
        life1_before,
        "p1's life is unchanged"
    );
    assert_eq!(
        engine.state().journal.len(),
        journal,
        "nothing is journaled"
    );
}

/// "{1}{G/P}, {T}, Sacrifice a creature: Search your library for a
/// creature card with mana value equal to 1 plus the sacrificed
/// creature's mana value, put that card onto the battlefield, then
/// shuffle. Activate only as a sorcery."
fn birthing_pod() -> CardIndex {
    card_index("f8b9dd54-0837-47f4-ad14-7a0322d46d5f")
}

/// Birthing Pod's Phyrexian symbol (CR 107.4f, CR 118.13a): paid by mana
/// or by 2 life, asked only where a board can afford it either way. The
/// Pod itself is gone by the time the answer is paid — the harness makes
/// it so, the way the other tests here make their own object so. The
/// activation is reversed (CR 732.1, 732.2): no life is paid, no mana is
/// spent, the Elves stand unsacrificed, and priority returns to the
/// activator.
#[test]
fn a_reversed_phyrexian_choice_pays_neither_life_nor_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(703, forest())
        .battlefield(0, &[birthing_pod(), forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pod = on_battlefield(&engine, p0, birthing_pod()).expect("the Pod");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 2, "two Forests");
    // The offer reads the pool as it stands, not what tapping could still
    // make (`Engine::can_pay_mana`), so the mana is floating before the
    // ability is even offered — the same order the Vault test above taps
    // its Forests in.
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(pool(&engine, p0), 2);
    let life_before = engine.state().players[p0.get() as usize].life;
    let journal = engine.state().journal.len();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.abilities.contains(&(pod, 0)), "{:?}", legal.abilities);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pod,
                ability_index: 0,
            },
        )
        .expect("the Pod activates");

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected the Phyrexian choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        matches!(prompt, crate::choice::YesNoPrompt::PayLife { amount: 2 }),
        "{prompt:?}"
    );
    cease(&mut engine, p0, pod, ZoneLocation::Battlefield);

    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("an answer the question offered is taken");

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activation is reversed and priority is the activator's: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was activated");
    assert_eq!(
        engine.state().players[p0.get() as usize].life,
        life_before,
        "no life is paid"
    );
    assert_eq!(
        pool(&engine, p0),
        2,
        "the floating mana is spent on nothing — the failure is ahead of \
         where payment reads the pool, and reversing it moves the pool no \
         further"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves stand, unsacrificed"
    );
    for forest in forests {
        assert!(
            engine
                .state()
                .object(forest)
                .expect("the Forest")
                .status
                .contains(crate::object::Status::TAPPED),
            "each Forest is exactly as tapping it for the still-floating \
             mana above left it"
        );
    }
    assert_eq!(
        engine.state().journal.len(),
        journal,
        "nothing is journaled"
    );
}
