//! `cards/artifacts/mv_4/tawnos_s_coffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit::{bears, power_toughness};

fn tawnos_s_coffin() -> CardIndex {
    card_index("05f20087-416f-4928-a0ab-6d0d2ca2ad05")
}

fn divine_transformation() -> CardIndex {
    card_index("292e7135-8804-43f2-a486-51ef97b83f77")
}

fn find_in_exile(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Tawnos's Coffin: "{3}, {T}: Exile target creature and all Auras attached to
/// it. Note the number and kind of counters that were on that creature. When
/// this artifact leaves the battlefield or becomes untapped, return that
/// exiled card to the battlefield under its owner's control tapped with the
/// noted number and kind of counters on it. If you do, return the other exiled
/// cards to the battlefield under their owner's control attached to that
/// permanent."
#[test]
fn tawnos_s_coffin_exiles_creature_and_auras_then_returns_with_noted_counters_and_auras_on_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let plus_one = CounterKind::Plus {
        power: 1,
        toughness: 1,
    };
    let mut engine = Duel::new(44, plains())
        .battlefield(
            0,
            &[
                tawnos_s_coffin(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                bears(),
            ],
        )
        .hand(0, &[divine_transformation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches main phase");

    let bear = on_battlefield(&engine, p0, bears()).expect("Bears on battlefield");
    // Attach Divine Transformation (+3/+3) and put two +1/+1 counters on Grizzly Bears.
    cast_from_hand(&mut engine, p0, divine_transformation());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    set_counters(&mut engine, bear, plus_one, 2);
    engine.dev_state_mut(p0).unwrap().refresh_characteristics();
    assert_eq!(power_toughness(&engine, bear), (7, 7));

    // Activate Tawnos's Coffin ({3}, {T}) targeting Grizzly Bears.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tawnos_s_coffin(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Both Grizzly Bears and Divine Transformation are now in exile.
    assert!(on_battlefield(&engine, p0, bears()).is_none());
    assert!(on_battlefield(&engine, p0, divine_transformation()).is_none());
    assert!(find_in_exile(&engine, p0, bears()).is_some());
    assert!(find_in_exile(&engine, p0, divine_transformation()).is_some());

    // Advance to p0's next untap step; pass_until's default LeaveTapped answer
    // chooses not to leave Tawnos's Coffin tapped (empty Vec), so it untaps
    // and fires the delayed return trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && stack_is_empty(e)
    });
    let _ = p1;

    let returned_bears =
        on_battlefield(&engine, p0, bears()).expect("Grizzly Bears returned on untap");
    let returned_aura = on_battlefield(&engine, p0, divine_transformation())
        .expect("Divine Transformation returned on untap");
    assert!(
        is_tapped(&engine, returned_bears),
        "returned creature enters tapped"
    );
    assert_eq!(
        counters_on(&engine, returned_bears, plus_one),
        2,
        "noted +1/+1 counters restored on return"
    );
    assert_eq!(
        engine
            .state()
            .object(returned_aura)
            .expect("aura")
            .attached_to,
        Some(returned_bears),
        "returned Aura is attached to the returned creature"
    );
    assert_eq!(
        power_toughness(&engine, returned_bears),
        (7, 7),
        "2/2 base + two +1/+1 counters + Divine Transformation (+3/+3) = 7/7"
    );
}
