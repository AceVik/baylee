//! "Your next untap step" names the ability's controller (CR 109.5),
//! independently of later control changes to the affected permanent.

use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier, TargetSpec};

mod cards;
pub(in crate::engine) use cards::assert_untaps_after_control_change;

const VICTIM: u32 = 7410;
const LOCKER: u32 = 7411;
const STEWARD: u32 = 7412;
const CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
static LOCKS: &[AbilityDef] = &[
    free(
        &[
            Effect::TapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::DoesNotUntap,
                Duration::UntilYourNextUntapStep,
            ),
        ],
        Some(TargetReq::one(CREATURE)),
    ),
    free(
        &[
            Effect::TapTarget,
            Effect::SuppressUntap {
                target: CREATURE,
                steps: 1,
            },
        ],
        Some(TargetReq::one(CREATURE)),
    ),
];
static STEWARDSHIP: &[AbilityDef] = &[
    // Untargeted so the same fixture can take a creature with shroud.
    free(
        &[Effect::continuous(
            &Filter::CREATURE,
            Modifier::GainControl,
            Duration::Indefinitely,
        )],
        None,
    ),
    free(&[Effect::TapTarget], Some(TargetReq::one(CREATURE))),
];

fn fixture(locker: u32) -> (Bench, ObjectId, ObjectId) {
    let mut engine = bench(
        7410,
        vec![
            card(
                VICTIM,
                creature_face("Victim", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                LOCKER,
                face("Locker", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                LOCKS,
            ),
            card(
                STEWARD,
                face("Steward", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                STEWARDSHIP,
            ),
        ],
        [
            Seat::with(&[locker, VICTIM, STEWARD]),
            Seat::with(&[STEWARD]),
        ],
    );
    to_main(&mut engine, me());
    let victim = the(&engine, ZoneLocation::Battlefield, VICTIM);
    let locker = the(&engine, ZoneLocation::Battlefield, locker);
    (engine, locker, victim)
}

fn target(engine: &mut Bench, seat: PlayerId, source: ObjectId, ability: u32, target: ObjectId) {
    activate(engine, seat, source, ability);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    settle(engine, seat);
}

fn steward(engine: &mut Bench, seat: PlayerId) -> ObjectId {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    objects(engine, ZoneLocation::Battlefield, STEWARD)
        .into_iter()
        .find(|id| engine.state().object(*id).unwrap().controller == seat)
        .unwrap()
}

fn take(engine: &mut Bench, seat: PlayerId, victim: ObjectId) {
    let source = steward(engine, seat);
    activate(engine, seat, source, 0);
    settle(engine, seat);
    assert_eq!(engine.state().object(victim).unwrap().controller, seat);
}

fn tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(crate::object::Status::TAPPED)
}

#[test]
fn your_next_untap_waits_for_you_after_another_player_untaps_the_creature() {
    let (mut engine, locker, victim) = fixture(LOCKER);
    target(&mut engine, me(), locker, 0, victim);
    take(&mut engine, them(), victim);
    to_main(&mut engine, them());
    assert!(
        !tapped(&engine, victim),
        "your restriction does not block the new controller"
    );
    take(&mut engine, me(), victim);
    let source = steward(&mut engine, me());
    target(&mut engine, me(), source, 1, victim);
    to_main(&mut engine, me());
    assert!(
        tapped(&engine, victim),
        "the other player's untap did not spend your restriction"
    );
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        !tapped(&engine, victim),
        "exactly one of your untap steps was suppressed"
    );
}

#[test]
fn your_next_untap_expires_even_while_somebody_else_controls_the_creature() {
    let (mut engine, locker, victim) = fixture(LOCKER);
    target(&mut engine, me(), locker, 0, victim);
    take(&mut engine, them(), victim);
    to_main(&mut engine, them());
    assert!(!tapped(&engine, victim));
    let source = steward(&mut engine, them());
    target(&mut engine, them(), source, 1, victim);
    to_main(&mut engine, me());
    take(&mut engine, me(), victim);
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        !tapped(&engine, victim),
        "your awaited step already passed while it was away"
    );
}

#[test]
fn an_affected_controller_restriction_still_follows_control_changes() {
    let (mut engine, locker, victim) = fixture(LOCKER);
    target(&mut engine, me(), locker, 1, victim);
    take(&mut engine, them(), victim);
    to_main(&mut engine, them());
    assert!(tapped(&engine, victim));
    to_main(&mut engine, me());
    to_main(&mut engine, them());
    assert!(!tapped(&engine, victim));
}
