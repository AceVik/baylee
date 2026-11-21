//! `cards/sorceries/mv_1/kaervek_s_torch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kaervek_s_torch() -> CardIndex {
    card_index("c629feae-084c-452f-9c09-4b9716331498")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Kaervek's Torch (c629feae-084c-452f-9c09-4b9716331498) — {X}{R} Sorcery:
/// "As long as Kaervek's Torch is on the stack, spells that target it cost {2}
/// more to cast. Kaervek's Torch deals X damage to any target."
/// With only {U}{U}, Counterspell cannot afford the {2}{U}{U} taxed cost, and
/// the Torch resolves for X = 3 damage.
#[test]
fn kaervek_s_torch_blocks_untaxed_counterspell_and_deals_x_damage() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[island(), island()],
        &[kaervek_s_torch()],
        &[counterspell()],
    );

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, kaervek_s_torch(), 0, 3, &[], &[P1]);
    let torch_on_stack = on_stack(&engine, kaervek_s_torch()).expect("Torch on stack");
    lk::pass_once(&mut engine);

    tap_all_mana(&mut engine, P1);
    assert_eq!(lk::pool_total(&engine, P1), 2);
    let counter_id = in_hand(&engine, P1, counterspell()).expect("Counterspell in hand");
    engine
        .apply(P1, PlayerAction::CastSpell { card: counter_id })
        .unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![torch_on_stack],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        lk::zone_of(&engine, counter_id),
        Zone::Hand,
        "targeting Kaervek's Torch costs {{2}}{{U}}{{U}}, so {{U}}{{U}} reverses the cast to hand"
    );
    assert_eq!(lk::pool_total(&engine, P1), 2);

    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 17, "X = 3 damage to target player");
}

/// Kaervek's Torch — paying the extra {2} allows Counterspell to counter it
/// (spending all 4 mana), and uncountered it can also target a creature.
#[test]
fn kaervek_s_torch_charges_two_extra_mana_to_counter_and_hits_creatures() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[island(), island(), island(), island()],
        &[kaervek_s_torch()],
        &[counterspell()],
    );

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, kaervek_s_torch(), 0, 3, &[], &[P1]);
    let torch_on_stack = on_stack(&engine, kaervek_s_torch()).expect("Torch on stack");
    lk::pass_once(&mut engine);

    tap_all_mana(&mut engine, P1);
    assert!(lk::castable(&engine, P1, counterspell()));
    lk::announce(
        &mut engine,
        P1,
        counterspell(),
        0,
        0,
        &[torch_on_stack],
        &[],
    );
    assert_eq!(
        lk::pool_total(&engine, P1),
        0,
        "all 4 mana ({{2}}{{U}}{{U}}) were spent to target Kaervek's Torch"
    );
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 20, "countered Torch dealt no damage");

    let mut vs_creature = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[hill_giant()],
        &[kaervek_s_torch()],
        &[],
    );
    let giant = lk::obj(&vs_creature, P1, hill_giant());
    tap_all_mana(&mut vs_creature, P0);
    lk::announce(&mut vs_creature, P0, kaervek_s_torch(), 0, 3, &[giant], &[]);
    lk::settle(&mut vs_creature);
    assert_eq!(lk::zone_of(&vs_creature, giant), Zone::Graveyard);
}
