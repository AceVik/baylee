//! Island Sanctuary: "If you would draw a card during your draw step,
//! instead you may skip that draw. If you do, until your next turn, you
//! can't be attacked except by creatures with flying and/or islandwalk."
//!
//! A skip is a replacement effect (CR 614.10), asked as the draw step's draw
//! would be made (CR 504.1); the restriction is one on declaring attackers
//! (CR 508.1c) and outlives the card (its 2004-10-04 ruling).
#[allow(clippy::wildcard_imports)] // Shared real-card test vocabulary.
use super::*;
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::Modifier;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// p0 holds the Sanctuary; p1 holds Grizzly Bears, Serra Angel and two
/// Lords of Atlantis, each of which gives the other islandwalk.
fn table() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(7126, plains())
        .battlefield(0, &[index::ISLAND_SANCTUARY])
        .battlefield(
            1,
            &[
                index::GRIZZLY_BEARS,
                index::SERRA_ANGEL,
                index::LORD_OF_ATLANTIS,
                index::LORD_OF_ATLANTIS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn offered(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            ..
        } if *player == P0
    )
}

fn hand_size(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

fn p1_creatures(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.card.map(|c| c.index) == Some(card))
        })
        .collect()
}

fn restricted(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, Modifier::CantBeAttackedExceptBy { .. }))
}

/// To p0's draw step with the skip asked. The player on the play has no
/// turn-1 draw step at all (CR 103.8a), so the first question is turn 3's.
fn to_offer(engine: &mut Engine<RegistryLookup>) -> usize {
    pass_until(engine, offered);
    assert_eq!(engine.state().turn.active, P0);
    assert_eq!(engine.state().turn.step, crate::turn::Step::Draw);
    hand_size(engine)
}

fn to_p1_attack(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    attackers
}

/// Yes: no card is drawn, and on p1's turn the Bears cannot attack p0 while
/// the Angel (flying) and each Lord (islandwalk from the other) can. A
/// declaration naming the Bears is refused; the Angel's is taken. On p0's
/// next turn the restriction has ended and the skip is asked again.
#[test]
fn island_sanctuary_skips_the_draw_and_lets_only_flyers_and_islandwalkers_attack() {
    let mut engine = table();
    let before = to_offer(&mut engine);
    assert_eq!(engine.state().turn.number, 3, "turn 1 is never asked");
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(hand_size(&engine), before, "the draw was skipped");
    assert!(restricted(&engine));

    let bears = p1_creatures(&engine, index::GRIZZLY_BEARS)[0];
    let angel = p1_creatures(&engine, index::SERRA_ANGEL)[0];
    let lords = p1_creatures(&engine, index::LORD_OF_ATLANTIS);
    let attackers = to_p1_attack(&mut engine);
    assert!(
        !attackers.contains(&bears),
        "the Bears have neither: {attackers:?}"
    );
    assert!(attackers.contains(&angel), "flying");
    assert!(lords.iter().all(|l| attackers.contains(l)), "islandwalk");
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(bears, Defender::Player(P0))],
                },
            )
            .is_err(),
        "the Bears can't attack p0"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(P0))],
            },
        )
        .unwrap();

    to_offer(&mut engine);
    assert_eq!(engine.state().turn.number, 5);
    assert!(!restricted(&engine), "until your next turn, and no longer");
}

/// No: the draw is made as it would have been, and the Bears attack.
#[test]
fn island_sanctuary_declined_draws_and_restricts_nothing() {
    let mut engine = table();
    let before = to_offer(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(hand_size(&engine), before + 1, "drawn after all");
    assert!(!restricted(&engine));
    let bears = p1_creatures(&engine, index::GRIZZLY_BEARS)[0];
    assert!(to_p1_attack(&mut engine).contains(&bears));
}

/// "The effect will continue until your next turn even if this card leaves
/// the battlefield."
#[test]
fn island_sanctuary_restriction_outlives_the_sanctuary() {
    let mut engine = table();
    to_offer(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let sanctuary = on_battlefield(&engine, P0, index::ISLAND_SANCTUARY).expect("in play");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            sanctuary,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    let bears = p1_creatures(&engine, index::GRIZZLY_BEARS)[0];
    assert!(!to_p1_attack(&mut engine).contains(&bears));
}
