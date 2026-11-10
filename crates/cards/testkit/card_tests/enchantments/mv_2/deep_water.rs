//! `cards/enchantments/mv_2/deep_water.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn deep_water_card() -> CardIndex {
    card_index("cf9ef041-5396-4f18-8233-42d667db0888")
}

fn board() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                deep_water_card(),
                island(),
                forest(),
                mountain(),
                swamp(),
                plains(),
                sol_ring(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn tap_for_mana(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    let source = on_battlefield(e, P0, card).expect("the mana source is out");
    e.apply(P0, PlayerAction::ActivateManaAbility { source })
        .expect("the mana ability is offered");
}

fn available(e: &Engine<RegistryLookup>, color: ManaColor) -> u32 {
    e.state.players[0].mana_pool.available(color)
}

/// Pays {U} off the Island and activates Deep Water.
fn turn_deep_water_on(e: &mut Engine<RegistryLookup>) {
    tap_for_mana(e, island());
    assert_eq!(available(e, ManaColor::Blue), 1);
    activate(e, P0, deep_water_card(), 0);
    pass_until(e, stack_is_empty);
    assert_eq!(available(e, ManaColor::Blue), 0, "the {{U}} was the cost");
}

/// Deep Water (cf9ef041-5396-4f18-8233-42d667db0888) — "{U}: Until end of
/// turn, if you tap a land you control for mana, it produces {U} instead of
/// any other type." Control: before it is activated, lands make their own
/// colours.
#[test]
fn deep_water_changes_every_other_land_type_to_blue_once_activated() {
    let mut e = board();
    tap_for_mana(&mut e, forest());
    assert_eq!(
        available(&e, ManaColor::Green),
        1,
        "control: a Forest makes {{G}}"
    );
    assert_eq!(available(&e, ManaColor::Blue), 0);

    // Fresh board, now with the effect on.
    let mut e = board();
    turn_deep_water_on(&mut e);
    for (land, was) in [
        (forest(), ManaColor::Green),
        (mountain(), ManaColor::Red),
        (swamp(), ManaColor::Black),
        (plains(), ManaColor::White),
    ] {
        let blue_before = available(&e, ManaColor::Blue);
        tap_for_mana(&mut e, land);
        assert_eq!(
            available(&e, ManaColor::Blue),
            blue_before + 1,
            "the {was:?} land now makes {{U}}"
        );
        assert_eq!(available(&e, was), 0, "and none of its own colour");
    }
}

/// Deep Water — "if you tap a land you control": a nonland mana source is
/// not changed.
#[test]
fn deep_water_leaves_a_nonland_mana_source_alone() {
    let mut e = board();
    turn_deep_water_on(&mut e);
    // Sol Ring's {T}: Add {C}{C} is a printed mana ability, offered as its
    // ability 0 rather than as a land's intrinsic mana.
    let ring = on_battlefield(&e, P0, sol_ring()).expect("Sol Ring is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .expect("Sol Ring taps for mana");
    assert_eq!(
        available(&e, ManaColor::Colorless),
        2,
        "Sol Ring is not a land: still {{C}}{{C}}"
    );
    assert_eq!(available(&e, ManaColor::Blue), 0);
}

/// Deep Water — "Until end of turn": on the next turn the Forest makes {G}
/// again.
#[test]
fn deep_water_wears_off_at_end_of_turn() {
    let mut e = board();
    turn_deep_water_on(&mut e);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    tap_for_mana(&mut e, forest());
    assert_eq!(available(&e, ManaColor::Green), 1, "{{G}} again");
    assert_eq!(available(&e, ManaColor::Blue), 0);
}
