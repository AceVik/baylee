#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn underdome_commits_a_real_un_spell_and_rejects_the_same_black_spell() {
    for is_un in [false, true] {
        let mut e = prepared(&[index::SOL_RING], &[index::UNDERDOME], &[]);
        let card = in_hand(&e, P0, index::SOL_RING).unwrap();
        print_fact(&mut e, card, is_un);
        pool(&mut e, P0, &[]);
        restricted(&mut e, P0, ManaColor::Blue);
        assert_eq!(restricted_units(&e, P0), 1);
        if is_un {
            cast_paid(&mut e, card);
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        } else {
            reject_cast(&mut e, card);
            assert_eq!(restricted_units(&e, P0), 1);
        }
    }
}
#[test]
fn underdome_pays_only_the_un_imposed_generic_tax_on_an_ordinary_spell() {
    for silver_tax in [false, true] {
        let mut e = prepared(
            &[index::WIND_DRAKE],
            &[index::UNDERDOME, index::BLACK_LOTUS],
            &[],
        );
        let spell = in_hand(&e, P0, index::WIND_DRAKE).unwrap();
        let tax = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
        print_fact(&mut e, spell, false);
        print_fact(&mut e, tax, silver_tax);
        impose(&mut e, tax, Modifier::SpellsCostMore(1));
        // Exclude future Lotus production from offers: this is a cost-source
        // provenance fixture, not a mana source in this scenario.
        e.state.set_tapped(tax, true);
        pool(
            &mut e,
            P0,
            &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        );
        restricted(&mut e, P0, ManaColor::Red);
        if silver_tax {
            cast_paid(&mut e, spell);
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        } else {
            reject_cast(&mut e, spell);
            assert_eq!(restricted_units(&e, P0), 1);
        }
    }
}
#[test]
fn underdome_tax_credit_cannot_spill_into_the_ordinary_spell_base_cost() {
    let mut e = prepared(
        &[index::WIND_DRAKE],
        &[index::UNDERDOME, index::BLACK_LOTUS],
        &[],
    );
    let spell = in_hand(&e, P0, index::WIND_DRAKE).unwrap();
    let tax = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
    print_fact(&mut e, spell, false);
    print_fact(&mut e, tax, true);
    impose(&mut e, tax, Modifier::SpellsCostMore(1));
    e.state.set_tapped(tax, true);
    pool(
        &mut e,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
    );
    restricted(&mut e, P0, ManaColor::Red);
    // Transparent untap only supplies a second genuine Underdome activation.
    let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
    e.state.set_tapped(land, false);
    e.refresh_offer();
    restricted(&mut e, P0, ManaColor::Red);
    assert_eq!(e.state.players[0].mana_pool.total(), 4);
    reject_cast(&mut e, spell);
    assert_eq!(restricted_units(&e, P0), 2);
}
#[test]
fn underdome_un_base_eligibility_does_not_cover_an_ordinary_imposed_tax() {
    for ordinary_unit in [false, true] {
        let mut e = prepared(
            &[index::SOL_RING],
            &[index::UNDERDOME, index::BLACK_LOTUS],
            &[],
        );
        let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
        let tax = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
        print_fact(&mut e, spell, true);
        print_fact(&mut e, tax, false);
        impose(&mut e, tax, Modifier::SpellsCostMore(1));
        e.state.set_tapped(tax, true);
        pool(
            &mut e,
            P0,
            if ordinary_unit {
                &[(ManaColor::Colorless, 1)]
            } else {
                &[]
            },
        );
        restricted(&mut e, P0, ManaColor::Red);
        if !ordinary_unit {
            let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
            e.state.set_tapped(land, false);
            e.refresh_offer();
            restricted(&mut e, P0, ManaColor::Red);
        }
        if ordinary_unit {
            cast_paid(&mut e, spell);
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        } else {
            reject_cast(&mut e, spell);
            assert_eq!(restricted_units(&e, P0), 2);
        }
    }
}
#[test]
fn underdome_locked_un_tax_survives_its_imposing_source_sacrificed_for_mana() {
    let mut e = prepared(
        &[index::WIND_DRAKE],
        &[index::UNDERDOME, index::BLACK_LOTUS],
        &[],
    );
    let spell = in_hand(&e, P0, index::WIND_DRAKE).unwrap();
    let lotus = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
    print_fact(&mut e, spell, false);
    print_fact(&mut e, lotus, true);
    impose(&mut e, lotus, Modifier::SpellsCostMore(1));
    pool(&mut e, P0, &[]);
    restricted(&mut e, P0, ManaColor::Red);
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player: P0, .. }),
        "actual casting payment opportunity"
    );
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: lotus,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(e.state.object(lotus).unwrap().zone, Zone::Graveyard);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "locked total4 and its tax quota survive source departure"
    );
}
