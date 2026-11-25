//! `cards/creatures/mv_6/darigaaz_the_igniter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_engine::event::{DamageTarget, GameEvent};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn darigaaz_the_igniter() -> CardIndex {
    card_index("6cbafa7b-4448-4ad4-b4f2-0ab6e7c95b0b")
}

fn yavimaya_kavu() -> CardIndex {
    card_index("285c9f49-4aeb-468c-bb41-f5ccb5257e2b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Darigaaz on the battlefield and a hand with two green cards, one of them
/// multicolored, one red card, and a colorless Forest.
fn game() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[darigaaz_the_igniter()])
        .hand(
            1,
            &[grizzly_bears(), yavimaya_kavu(), lightning_bolt(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, darigaaz_the_igniter());
    pass_until(&mut engine, stack_is_empty);
    let darigaaz = on_battlefield(&engine, P0, darigaaz_the_igniter()).expect("Darigaaz entered");
    (engine, darigaaz)
}

/// Attack on Darigaaz's second turn and reach its optional mana payment.
fn offer_payment(engine: &mut Engine<RegistryLookup>, darigaaz: ObjectId) {
    reach_their_main_phase(engine, P1);
    reach_their_main_phase(engine, P0);
    let _ = attack_and_collect_blocks(engine, darigaaz, P1);
    pass_until(engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(engine, P0);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
}

fn pay_and_choose(engine: &mut Engine<RegistryLookup>, color: ManaColor) {
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
    pass_until(engine, |e| e.state().turn.phase == Phase::SecondMain);
}

fn hand_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

fn reveals(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::Revealed { player, .. } if player == seat))
        .count()
}

fn revealed(engine: &Engine<RegistryLookup>, seat: PlayerId, hand: &[ObjectId]) -> bool {
    engine.state().journal.entries().iter().any(|entry| {
        matches!(&entry.event, GameEvent::Revealed { player, cards } if *player == seat && cards == hand)
    })
}

/// Darigaaz, the Igniter (6cbafa7b-4448-4ad4-b4f2-0ab6e7c95b0b) — Flying;
/// on combat damage to a player, pay `{2}{R}` to choose a color, reveal that
/// player's hand, and deal one additional damage per revealed card of that color.
#[test]
fn darigaaz_counts_monocolored_and_multicolored_cards_and_deals_noncombat_damage() {
    let (mut engine, darigaaz) = game();
    assert_eq!(pt(&engine, darigaaz), (6, 6));
    assert!(keywords(&engine, darigaaz).contains(KeywordSet::FLYING));
    offer_payment(&mut engine, darigaaz);
    let theirs = hand_of(&engine, P1);
    pay_and_choose(&mut engine, ManaColor::Green);

    assert_eq!(
        engine.state().players[1].life,
        12,
        "6 combat damage plus Bears and the red-green Kavu"
    );
    assert!(
        revealed(&engine, P1, &theirs),
        "the whole hand was revealed"
    );
    assert_eq!(hand_of(&engine, P1), theirs, "nothing is discarded");
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::DamageDealt {
                    source: Some(source),
                    target: DamageTarget::Player(P1),
                    amount: 2,
                    is_combat: false,
                } if source == darigaaz
            )),
        "Darigaaz itself deals the noncombat damage"
    );
}

#[test]
fn darigaaz_declining_payment_reveals_nothing_and_deals_no_additional_damage() {
    let (mut engine, darigaaz) = game();
    let reveals_before = reveals(&engine, P1);

    offer_payment(&mut engine, darigaaz);
    let theirs = hand_of(&engine, P1);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 14, "only combat damage");
    assert_eq!(reveals(&engine, P1), reveals_before);
    assert_eq!(hand_of(&engine, P1), theirs);
}

#[test]
fn darigaaz_reveals_the_hand_but_deals_zero_without_the_chosen_color() {
    let (mut engine, darigaaz) = game();
    offer_payment(&mut engine, darigaaz);
    let theirs = hand_of(&engine, P1);
    pay_and_choose(&mut engine, ManaColor::White);

    assert_eq!(engine.state().players[1].life, 14, "only combat damage");
    assert!(revealed(&engine, P1, &theirs));
    assert_eq!(hand_of(&engine, P1), theirs);
}
