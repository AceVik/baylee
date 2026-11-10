//! `cards/lands/utility/iron_hills.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Iron Hills enters tapped, taps for {R} or {W}, and — for {2}{R}{W}, its own
/// tap and its own sacrifice — puts two +1/+1 counters on a Dwarf you control,
/// at sorcery speed only.
///
/// The Dwarf across the table is the control for `Filter::YOUR_…`: a body the
/// ability must decline even though it is a Dwarf, so the counters land on the
/// host and nowhere else. The land itself is asked to be on its own menu in the
/// way this pool's sacrifice costs are: what the ability eats is its own source.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn iron_hills_taps_for_red_or_white_and_eats_itself_to_grow_a_dwarf() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The Dwarves: one under p0 (the target), one under p1 (the control).
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                plains(),
                iron_hills(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // Enters tapped: a land played on turn one, on an empty board, has
    // nothing standing between it and play.
    let (mut engine, hills) = {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, forest())
            .hand(0, &[iron_hills()])
            .battlefield(0, &[mountain(), mountain(), plains(), llanowar_elves()])
            .battlefield(1, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let land = play_land(&mut engine, p0, iron_hills());
        (engine, land)
    };
    assert!(entered_tapped(&engine, hills), "this land enters tapped");

    // And therefore nothing it prints is on offer yet: both halves of Iron
    // Hills cost `{T}`, so the turn it arrives it is a land that does
    // nothing at all. Its next turn is where the card is readable.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, hills), "the untap step stood it up");

    // The two halves of the mana line are a choice, and the choice is not a
    // default: {R} and {W} are what it offers, and {G} is not.
    activate(&mut engine, p0, iron_hills(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("`{{R}} or {{W}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::White],
        "red or white, and the land prints nothing else"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the colour that was named"
    );

    // The other half needs a board of its own, and for two reasons that are
    // both the card's: `{2}{R}{W}, {T}, Sacrifice this land` wants the Hills
    // *untapped*, which the mana line above just spent, and it wants a
    // **Dwarf you control** to point at, which no Elf is. Placed rather than
    // played, so it stands up; the entry clause was read at the top.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                plains(),
                plains(),
                iron_hills(),
                dwarven_armorer(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), dwarven_armorer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Hills are kept back: their `{T}` is the ability's own.
    tap_all_mana_but(&mut engine, p0, Some(iron_hills()));
    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red)
            >= 1
            && engine.state().players[0]
                .mana_pool
                .available(ManaColor::White)
                >= 1,
        "the pool can pay the coloured half: {:?}",
        engine.state().players[0].mana_pool
    );
    activate(&mut engine, p0, iron_hills(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks for a Dwarf you control, got {:?}",
            engine.pending()
        )
    };
    let dwarf = on_battlefield(&engine, p0, dwarven_armorer()).expect("a Dwarf of your own");
    assert!(options.contains(&dwarf), "your Dwarf is the answer");
    assert!(
        !options.contains(&on_battlefield(&engine, p1, llanowar_elves()).unwrap()),
        "an Elf is no Dwarf: {options:?}"
    );
    assert!(
        !options.contains(&on_battlefield(&engine, p1, dwarven_armorer()).unwrap()),
        "and a Dwarf across the table is not one you control: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dwarf],
            },
        )
        .expect("a Dwarf you control is the whole of the answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, iron_hills()).is_none(),
        "the land sacrificed itself to pay"
    );
    assert_eq!(
        counters_on(&engine, dwarf, CounterKind::P1P1),
        2,
        "two +1/+1 counters, and the colour choice never decided them"
    );
}
