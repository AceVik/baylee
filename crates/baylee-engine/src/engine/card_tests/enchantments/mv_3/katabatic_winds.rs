//! `cards/enchantments/mv_3/katabatic_winds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn katabatic_winds() -> CardIndex {
    card_index("70446c56-07de-4d7f-94ae-4da88fba41cf")
}

/// Whether `id` is phased out.
fn phased_out(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::PHASED_OUT)
}

/// `Katabatic Winds` ({2}{G}, Enchantment, `Coverage::Partial`): creatures with flying can't
/// attack. A creature without flying attacks as usual.
#[test]
fn katabatic_winds_prevents_flying_creatures_from_attacking_but_not_the_others() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), serra_angel(), grizzly_bears()],
        )
        .hand(0, &[katabatic_winds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, katabatic_winds());
    pass_until(&mut engine, stack_is_empty);
    let winds = on_battlefield(&engine, p0, katabatic_winds()).unwrap();
    assert!(keywords(&engine, winds).contains(KeywordSet::PHASING));
    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(angel, Defender::Player(p1))],
                },
            )
            .is_err(),
        "creature with flying cannot attack while Katabatic Winds is on the battlefield"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .expect("a creature without flying attacks as usual");
}

/// Both players' fliers are held: the opponent's Air Elemental cannot attack and p0's Serra Angel
/// cannot block, while the opponent's ground creature attacks and is then blocked by nothing.
#[test]
fn katabatic_winds_stops_every_flying_creature_from_attacking_or_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), serra_angel()])
        .battlefield(1, &[air_elemental(), grizzly_bears()])
        .hand(0, &[katabatic_winds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, katabatic_winds());
    pass_until(&mut engine, stack_is_empty);
    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let air = on_battlefield(&engine, p1, air_elemental()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(air, Defender::Player(p0))],
                },
            )
            .is_err(),
        "the opponent's flier cannot attack either"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(angel, bear)],
                },
            )
            .is_err(),
        "a creature with flying cannot block"
    );
}

/// "Phasing": the enchantment phases out in its controller's next untap step, and while it is
/// phased out it does not exist, so the Angel may attack again.
#[test]
fn katabatic_winds_phases_out_and_lets_fliers_attack_while_it_is_gone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), serra_angel()])
        .hand(0, &[katabatic_winds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, katabatic_winds());
    pass_until(&mut engine, stack_is_empty);
    let winds = on_battlefield(&engine, p0, katabatic_winds()).unwrap();
    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    assert!(!phased_out(&engine, winds));

    reach_their_main_phase(&mut engine, p1);
    assert!(
        !phased_out(&engine, winds),
        "only its controller's untap phases it"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        phased_out(&engine, winds),
        "phased out in its controller's untap step"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1))],
            },
        )
        .expect("Winds is phased out: the Angel may attack");
}
