//! `cards/creatures/mv_2/devoted_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Devoted Druid ({1}{G}, 0/2): `{T}: Add {G}.` and `Put a -1/-1 counter on
/// this creature: Untap this creature.`
///
/// Two firsts in one card, and they are two halves of the same sentence.
/// `CostPart::PutCounterSelf` is a cost that puts a counter **on** the thing
/// paying it, which is the direction the engine had no door for — every
/// counter cost until now took one off — and `Effect::UntapSelf` is an untap
/// that names no target, so nobody is asked anything (CR 115.1c makes an
/// activated ability targeted only when it says the word).
///
/// **The bound is the counter and not the cost.** `can_afford` has nothing
/// to refuse here: a permanent can always take a counter, so the ability is
/// offered whatever has happened to the Druid. What stops it is the Druid
/// itself, and the moment it stops is sharper than "eventually": a cost is
/// paid as the ability is activated (CR 602.2b), state-based actions run
/// before anybody gets priority again (CR 704.3), and a creature with
/// toughness 0 is put into its owner's graveyard (CR 704.5f). So the second
/// payment kills the Druid **before the untap it just bought resolves** —
/// the ability is on the stack with nothing left to untap.
///
/// Each step is asserted rather than the total, because the total is the one
/// number that would still be right if the cost did nothing: a Druid whose
/// counters never landed would make two green as well, and then keep going.
#[test]
fn devoted_druid_untaps_itself_until_its_own_counters_kill_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1201, forest())
        .battlefield(0, &[devoted_druid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let druid = on_battlefield(&engine, p0, devoted_druid()).expect("the Druid is on the table");
    assert_eq!(pt(&engine, druid), (0, 2), "a 0/2 to start with");

    // Round one: tap for green, pay a counter, untap.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("an untapped Druid taps for green");
    assert!(is_tapped(&engine, druid));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("a permanent can always take a counter, so this is never refused");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "`Untap this creature` names no target, so nobody is asked anything: {:?}",
        engine.pending()
    );
    assert_eq!(
        counters_on(&engine, druid, CounterKind::M1M1),
        1,
        "the counter lands as the cost is paid, before the ability resolves"
    );
    assert_eq!(
        pt(&engine, druid),
        (-1, 1),
        "so the Druid is already smaller"
    );
    assert!(
        is_tapped(&engine, druid),
        "and still tapped, because what it paid for is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, druid), "which then untaps it");

    // Round two: the same again, and the payment is what kills it.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("an untapped Druid taps for green a second time");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "two green, which is the whole of what an untouched Druid is worth"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("still never refused — a 0/1 can take a counter like anything else");
    assert!(
        in_graveyard(&engine, p0, devoted_druid()).is_some(),
        "the second -1/-1 leaves a 0/0, and CR 704.5f takes it away before \
         priority comes back — so the untap it paid for is on the stack with \
         nothing left to untap"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "and the pool keeps what a creature that has since died put into it"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == druid),
        "a Druid in a graveyard is offered nothing: {:?}",
        legal.abilities
    );
}

/// The same payment under a Doubling Season, which does **not** double it.
///
/// CR 614.16 is the whole of this test. A replacement effect written "if an
/// effect would put one or more counters on a permanent" applies to what the
/// effect of a resolving spell or ability puts there, and to what another
/// replacement or prevention effect puts there — **paying a cost is
/// neither**. Doubling Season is the pool's only such replacement and prints
/// exactly that wording, so the answer is measured off the card rather than
/// chosen to be safe.
///
/// It matters by two counters: a doubled payment would put a 0/2 Druid in
/// the graveyard on its *first* untap, turning a card that makes two green
/// into one that makes one. So the counter-test is here too — the enchantment
/// is on the battlefield and doing its job on a permanent that *enters* with
/// counters is asserted nowhere near here, but the Druid's own line has to
/// come out at one.
#[test]
fn a_counter_paid_as_a_cost_is_not_doubled() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1203, forest())
        .battlefield(0, &[devoted_druid(), doubling_season()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let druid = on_battlefield(&engine, p0, devoted_druid()).expect("the Druid is on the table");
    assert!(
        on_battlefield(&engine, p0, doubling_season()).is_some(),
        "and the Season is out, which is the whole premise"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 0,
            },
        )
        .expect("tap for green");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: druid,
                ability_index: 1,
            },
        )
        .expect("pay a counter to untap");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, druid, CounterKind::M1M1),
        1,
        "one counter, not two: a cost is not an effect, so the Season's \
         `if an effect would put` never applies to it (CR 614.16)"
    );
    assert_eq!(
        pt(&engine, druid),
        (-1, 1),
        "and the Druid is a -1/1 rather than the -2/0 a doubled payment \
         would have killed"
    );
    assert!(
        in_graveyard(&engine, p0, devoted_druid()).is_none(),
        "so it is still on the battlefield and still worth a second green"
    );
}
