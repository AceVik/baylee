#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::KeywordSet;

#[test]
fn aegis_and_banner_apply_their_rarity_stats_and_each_new_keyword() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        for family in [Family::DawnAegis, Family::SwarmBanner] {
            let mut game = opening(
                &[item(family, rarity), item(Family::Grove, Rarity::Common)],
                &["Memnite"],
            );
            cast_free(&mut game, "Memnite");
            let tier = rarity.tier();
            for id in game.state().zones.list(ZoneLocation::Battlefield) {
                let object = game.state().arena.get(*id).unwrap();
                if object.controller != player {
                    continue;
                }
                let stats = object.characteristics();
                if !stats.types.contains(baylee_core::types::TypeSet::CREATURE) {
                    continue;
                }
                let applies = family == Family::DawnAegis || object.token.is_some();
                let (power, toughness) = if !applies {
                    (1, 1)
                } else if family == Family::DawnAegis {
                    (1, 1 + i16::from(tier))
                } else {
                    [(2, 1), (2, 2), (3, 2), (3, 3)][usize::from(tier - 1)]
                };
                assert_eq!(
                    (stats.power, stats.toughness),
                    (Some(power), Some(toughness))
                );
                assert_eq!(
                    stats.keywords.contains(KeywordSet::VIGILANCE),
                    applies && tier >= 2
                );
                assert_eq!(
                    stats.keywords.contains(KeywordSet::LIFELINK),
                    applies && tier >= if family == Family::DawnAegis { 3 } else { 4 }
                );
                assert_eq!(
                    stats.keywords.contains(KeywordSet::TRAMPLE),
                    family == Family::SwarmBanner && applies && tier >= 3
                );
                assert_eq!(
                    stats.keywords.contains(KeywordSet::HEXPROOF),
                    family == Family::DawnAegis && tier == 4
                );
            }
        }
    }
}

#[test]
fn acorn_table_abilities_scale_without_becoming_a_removable_permanent() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::GildedAcorn, rarity);
        let mut game = prepared(&[definition], &[], 19);
        main(&mut game, 1, player);
        assert_eq!(
            game.state().players[0].life,
            20 + i32::from(rarity.tier() >= 2)
        );
        assert_eq!(plants(&game, player), usize::from(rarity.tier() >= 3));
        let id = source(&game, player, definition);
        assert_eq!(game.state().arena.get(id).unwrap().kind, ObjectKind::Emblem);
        if rarity == Rarity::MythicRare {
            for id in game.state().zones.list(ZoneLocation::Battlefield) {
                assert_eq!(
                    game.state().arena.get(*id).unwrap().characteristics().power,
                    Some(2)
                );
            }
        }
    }
}
