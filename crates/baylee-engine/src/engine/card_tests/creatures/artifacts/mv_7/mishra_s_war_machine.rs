//! `cards/creatures/artifacts/mv_7/mishra_s_war_machine.rs`, played.
//!
//! Mishra's War Machine prints two sentences: `Banding` and "At the beginning
//! of your upkeep, this creature deals 3 damage to you unless you discard a
//! card. If it deals damage to you this way, tap it." The second is two
//! rules at once: a price the controller may decline (a discard), and a
//! rider that is read off what the damage *did*, not off what the player
//! chose (CR 608.2c) -- which is what separates a discard, a taken hit and a
//! hit that a prevention effect swallowed.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

fn mishra_s_war_machine() -> CardIndex {
    card_index("18b1f96f-de35-4f0a-8b0e-81576471fd15")
}

fn healing_salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

/// A seat's current life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Passes priority through `p0`'s upkeep, answering whatever the War
/// Machine's trigger asks the way a player would: `discard` names the card
/// given up, `None` declines and takes the damage. Returns the menu of every
/// question the trigger put to `p0`, so a test can say whether it was asked
/// at all.
///
/// Stops the moment the upkeep is over, which is also the moment the trigger
/// has finished: a trigger that is still on the stack keeps the step open.
#[track_caller]
fn through_the_upkeep(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    discard: Option<CardIndex>,
) -> Vec<Vec<ObjectId>> {
    assert_eq!(engine.state().turn.active, p0, "p0's own upkeep");
    assert_eq!(
        engine.state().turn.step,
        Step::Upkeep,
        "the game opens in the upkeep, where the trigger is waiting"
    );
    let mut menus = Vec::new();
    for _ in 0..40 {
        if engine.state().turn.step != Step::Upkeep {
            return menus;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                assert_eq!(player, p0, "the controller decides whether to discard");
                engine
                    .apply(player, PlayerAction::YesNo(discard.is_some()))
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the controller chooses the card to give up");
                let chosen = discard
                    .and_then(|card| in_hand(engine, p0, card))
                    .filter(|id| options.contains(id))
                    .into_iter()
                    .collect();
                menus.push(options);
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                    .unwrap();
            }
            other => panic!("unexpected while the War Machine's trigger resolves: {other:?}"),
        }
    }
    panic!("the upkeep never ended");
}

/// "...unless you discard a card." Discarding is the way out: the card goes
/// to the graveyard, no damage is dealt, and because the War Machine dealt
/// none "If it deals damage to you this way, tap it" has nothing to do.
#[test]
fn mishra_s_war_machine_spares_its_controller_for_a_discarded_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine()])
        .hand(0, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    let hand_forest = in_hand(&engine, p0, forest()).expect("a Forest in hand");
    let hand_island = in_hand(&engine, p0, island()).expect("an Island in hand");
    let from = engine.journal().entries().len();

    let menus = through_the_upkeep(&mut engine, p0, Some(island()));

    assert_eq!(menus.len(), 1, "one question: pay the discard or not");
    assert!(
        menus[0].contains(&hand_forest) && menus[0].contains(&hand_island),
        "\"a card\" -- either card in hand may be the price: {:?}",
        menus[0]
    );
    assert_eq!(
        life_of(&engine, p0),
        20,
        "the discard bought the damage off"
    );
    assert!(
        damage_events(&engine, from).is_empty(),
        "no damage was dealt at all"
    );
    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "the discarded card is in the graveyard"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "and only that card: the other stays in hand"
    );
    assert!(
        !is_tapped(&engine, machine),
        "it dealt no damage to its controller, so it is not tapped"
    );
}

/// "...this creature deals 3 damage to you unless you discard a card. If it
/// deals damage to you this way, tap it." Declining the discard takes the
/// 3 from the War Machine itself, and the damage taps it.
#[test]
fn mishra_s_war_machine_deals_three_to_its_controller_and_taps_when_they_decline() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    let from = engine.journal().entries().len();

    let menus = through_the_upkeep(&mut engine, p0, None);

    assert_eq!(menus.len(), 1, "the discard was offered and declined");
    assert_eq!(life_of(&engine, p0), 17, "3 damage to its controller");
    assert_eq!(
        damage_events(&engine, from),
        vec![(machine, crate::event::DamageTarget::Player(p0), 3, false)],
        "the War Machine itself dealt exactly 3 noncombat damage to p0"
    );
    assert!(
        is_tapped(&engine, machine),
        "\"If it deals damage to you this way, tap it\""
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "declining kept the card"
    );
}

/// "You can't avoid taking damage if you have no cards to discard." (ruling,
/// 2004-10-04.) With an empty hand there is nothing to offer: the damage is
/// dealt, the War Machine taps, and no question is asked.
#[test]
fn mishra_s_war_machine_must_deal_its_damage_when_the_hand_is_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "the seat starts with no hand"
    );

    let menus = through_the_upkeep(&mut engine, p0, None);

    assert!(
        menus.is_empty(),
        "there is no card to give up, so no question"
    );
    assert_eq!(
        life_of(&engine, p0),
        17,
        "the 3 damage could not be avoided"
    );
    assert!(
        is_tapped(&engine, machine),
        "and it dealt damage, so it taps"
    );
}

/// "If it deals damage to you **this way**, tap it": the rider reads what the
/// damage did. A prevention effect that swallows all of the 3 means the War
/// Machine dealt none (CR 615.1), so it stays untapped even though its
/// controller declined, or had nothing, to discard.
///
/// Healing Salve is cast with the trigger on the stack, so the hand is empty
/// by the time the trigger resolves and there is nothing to offer.
#[test]
fn mishra_s_war_machine_stays_untapped_when_the_damage_is_prevented() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine(), plains()])
        .hand(0, &[healing_salve()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    pass_until(&mut engine, |e| !stack_is_empty(e));
    assert_eq!(
        engine.state().turn.step,
        Step::Upkeep,
        "the trigger is on the stack in the upkeep"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and its controller holds priority over it: {:?}",
        engine.pending()
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(1)))
        .expect("the prevention mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");

    let menus = through_the_upkeep(&mut engine, p0, None);

    assert!(menus.is_empty(), "the Salve was the hand's last card");
    assert_eq!(
        life_of(&engine, p0),
        20,
        "the shield prevented all 3 damage"
    );
    assert!(
        !is_tapped(&engine, machine),
        "no damage was dealt to its controller this way, so it is not tapped"
    );
}

/// "At the beginning of **your** upkeep": the trigger is its controller's
/// alone. The opponent's upkeep neither costs the controller a card nor
/// life.
#[test]
fn mishra_s_war_machine_ignores_the_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine()])
        .hand(0, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    let menus = through_the_upkeep(&mut engine, p0, Some(island()));
    assert_eq!(menus.len(), 1, "p0's own upkeep asked");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    // `pass_until` refuses any question, so this walks p1's upkeep and draw
    // and fails on the first thing the trigger might ask.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step != Step::Upkeep
    });

    assert_eq!(life_of(&engine, p0), 20, "p0 took nothing in p1's upkeep");
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "and discarded nothing"
    );
    assert!(
        !is_tapped(&engine, machine),
        "and the Machine stayed untapped"
    );
}

/// "Banding": the War Machine leads a band with a creature that has none --
/// it is asked who bands with it (CR 508.1e, 702.22c), and a named ally is in
/// its band afterwards. The keyword is checked on the projected object, not
/// on the printing.
#[test]
fn mishra_s_war_machine_has_banding_and_bands_with_an_ally() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_war_machine(), savannah_lions()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let machine = on_battlefield(&engine, p0, mishra_s_war_machine()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
    assert!(
        keywords(&engine, machine).contains(KeywordSet::BANDING),
        "the War Machine has banding"
    );
    // Pay the upkeep with the Forest so the Machine is untapped to attack.
    let menus = through_the_upkeep(&mut engine, p0, Some(forest()));
    assert_eq!(menus.len(), 1, "the upkeep trigger asked once");
    assert!(!is_tapped(&engine, machine), "paid for, so untapped");

    declare_band_attack(&mut engine, p0, p1, &[machine, lions]);
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the band question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the attacking player announces the band");
    assert_eq!(
        prompt,
        ChoicePrompt::Band { with: machine },
        "the question belongs to the creature with banding"
    );
    assert_eq!(options, vec![lions], "the other attacker is on offer");

    answer_band_and_reach_blockers(&mut engine, p0, vec![lions]);
    assert!(
        engine.state().combat.band_of(machine).is_some(),
        "the War Machine is in a band"
    );
    assert_eq!(
        engine.state().combat.band_of(machine),
        engine.state().combat.band_of(lions),
        "and the Lions are in the same one"
    );
}
