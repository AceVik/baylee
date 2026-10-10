//! The tests of the cards under `cards/lands/towns/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod adventurer_s_inn;
mod balamb_garden_see_d_academy;
mod baron_airship_kingdom;
mod capital_city;
mod clive_s_hideaway;
mod eden_seat_of_the_sanctum;
mod gohn_town_of_ruin;
mod gongaga_reactor_town;
mod guadosalam_farplane_gateway;
mod insomnia_crown_city;
mod ishgard_the_holy_see;
mod jidoor_aristocratic_capital;
mod lindblum_industrial_regency;
mod midgar_city_of_mako;
mod rabanastre_royal_city;
mod sharlayan_nation_of_scholars;
mod starting_town;
mod the_gold_saucer;
mod treno_dark_city;
mod vector_imperial_capital;
mod windurst_federation_center;
mod zanarkand_ancient_metropolis;

/// Supply mana for a spell test without adding lands to the board being counted.
fn fund_adventure(e: &mut Engine<RegistryLookup>, player: PlayerId, color: ManaColor, n: u32) {
    e.dev_state_mut(player).unwrap().players[player.get() as usize]
        .mana_pool
        .add(color, n);
    e.refresh_offer();
}

fn begin_town_adventure(
    e: &mut Engine<RegistryLookup>,
    player: PlayerId,
    index: CardIndex,
    color: ManaColor,
    n: u32,
) -> ObjectId {
    fund_adventure(e, player, color, n);
    let card = in_hand(e, player, index).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.castable.contains(&card),
        "the Adventure face must be offered"
    );
    e.apply(player, PlayerAction::CastSpell { card }).unwrap();
    card
}

fn assert_adventure_exiled(e: &Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(card).unwrap().face_index, 0);
    assert!(crate::casting::adventure_permission(
        e.state(),
        player,
        card
    ));
    assert!(types(e, card).contains(TypeSet::LAND));
}
