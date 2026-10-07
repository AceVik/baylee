//! `cards/instants/mv_4/clever_concealment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Any number of target nonland permanents you control phase out": every
/// target does, not the first. Holy Strength on the first Cleric is named as
/// well, and it phases out with the Cleric it enchants (CR 702.26g, 702.26h)
/// and phases in with it at seat 0's next untap step, still attached.
#[test]
fn clever_concealment_phases_out_every_target() {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(229, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                ondu_cleric(),
                ondu_cleric(),
                holy_strength(),
            ],
        )
        .start();
    let clerics = all_on_battlefield(&engine, seat, ondu_cleric());
    let aura = on_battlefield(&engine, seat, holy_strength()).expect("seated");
    {
        // What a starting battlefield cannot say, set before the first
        // state-based check would put the Aura into a graveyard.
        let state = engine.dev_state_mut(seat).expect("the harness sets up");
        state.object_mut(aura).expect("seated").attached_to = Some(clerics[0]);
        state.invalidate_projections();
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);

    let card = in_hand(&engine, seat, clever_concealment()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is offered");
    let named = vec![clerics[0], clerics[1], aura];
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the spell asks for its targets, got {:?}", engine.pending());
    };
    assert!(
        named.iter().all(|id| options.contains(id)),
        "both Clerics and the Aura are nonland permanents seat 0 controls: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: named.clone(),
                players: vec![],
            },
        )
        .expect("any number of targets");
    if tap_to_pay_question(&engine).is_some() {
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![],
                },
            )
            .expect("the Plains pay it all");
    }
    pass_until(&mut engine, stack_is_empty);
    let phased = |engine: &Engine<RegistryLookup>, id| {
        engine
            .state()
            .object(id)
            .is_some_and(|o| o.status.contains(crate::object::Status::PHASED_OUT))
    };
    assert!(phased(&engine, clerics[0]), "the first target phased out");
    assert!(
        phased(&engine, clerics[1]),
        "the second target stayed phased in"
    );
    assert!(phased(&engine, aura), "the Aura stayed phased in");
    let indirectly = |engine: &Engine<RegistryLookup>, id| {
        engine.state().object(id).is_some_and(|o| {
            o.status
                .contains(crate::object::Status::PHASED_OUT_INDIRECTLY)
        })
    };
    assert!(
        indirectly(&engine, aura),
        "the Aura was named and is on a Cleric that phased out: it phases out \
         indirectly (CR 702.26h)"
    );
    assert!(
        !indirectly(&engine, clerics[0]) && !indirectly(&engine, clerics[1]),
        "the Clerics phased out directly"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, seat);
    for id in &named {
        assert!(
            !phased(&engine, *id),
            "all three phase in at seat 0's untap step"
        );
    }
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(clerics[0]),
        "the Aura phased in attached to the Cleric it enchanted"
    );
}
