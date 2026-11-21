//! `cards/enchantments/mv_3/astral_slide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn astral_slide() -> CardIndex {
    card_index("70f14b10-8129-4249-a267-8b4be5d9befa")
}

fn secluded_steppe() -> CardIndex {
    card_index("8dec6fcf-1254-4b1b-ba23-7a3e492a7241")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Astral Slide exiles target creature when a player cycles and returns it to
/// the battlefield under its OWNER's control at the beginning of the next end
/// step, supports declining, and ignores plain discards.
#[test]
fn astral_slide_exiles_and_returns_under_owners_control_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                astral_slide(),
                llanowar_elves(),
                putrid_imp(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island(), plains()])
        .hand(0, &[secluded_steppe(), swamp()])
        .hand(1, &[control_magic(), secluded_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf seated");

    // Plain discard via Putrid Imp does not trigger Astral Slide.
    let swamp_card = in_hand(&engine, p0, swamp()).expect("Swamp in hand");
    activate(&mut engine, p0, putrid_imp(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_card],
            },
        )
        .expect("discard Swamp");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, elf), Zone::Battlefield);

    // P0 cycles and declines MayDo -> creature stays on battlefield.
    let white = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| id == white[0]);
    activate(&mut engine, p0, secluded_steppe(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, elf), Zone::Battlefield);

    // On P1's turn, P1 steals P0's Elf with Control Magic, then P1 cycles
    // Secluded Steppe -> P0's Astral Slide triggers and exiles the stolen Elf.
    reach_their_main_phase(&mut engine, p1);
    let blues = all_on_battlefield(&engine, p1, island());
    tap_mana_where(&mut engine, p1, |id| blues.contains(&id));
    cast_with_floating(&mut engine, p1, control_magic());
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(elf).unwrap().controller, p1);

    let p1_plains = on_battlefield(&engine, p1, plains()).expect("P1 Plains");
    tap_mana_where(&mut engine, p1, |id| id == p1_plains);
    activate(&mut engine, p1, secluded_steppe(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p1));

    assert_eq!(
        zone_of(&engine, elf),
        Zone::Exile,
        "Elf remains exiled during P1's main phase"
    );

    // Beginning of P1's end step returns the Elf under its OWNER's (P0's) control.
    to_step(&mut engine, p1, Step::End);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(zone_of(&engine, elf), Zone::Battlefield);
    assert_eq!(
        engine.state().object(elf).unwrap().controller,
        p0,
        "returned under owner's control, not Control Magic's controller"
    );
}
