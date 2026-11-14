use super::*;
use std::collections::BTreeMap;

#[test]
fn all_tower_routes_preserve_legal_working_decks_and_exact_offered_replacements() {
    let generator = Generator::from_source(&crate::working::repo_root()).unwrap();
    let period = Period::containing(Kind::Weekly, 1_794_182_400).unwrap();
    let theme = deck::theme(period);
    theme.validate().unwrap();
    deck::opponent_pool(&generator.working).unwrap();
    let cards = |counts| {
        let text = deck::player(&theme, counts, &generator.working).unwrap();
        let loaded = baylee_cards::decks::house_deck(&text).unwrap();
        let main = loaded
            .main
            .iter()
            .map(|card| card.index)
            .collect::<Vec<_>>();
        assert_eq!(main.len(), 60);
        assert!(
            baylee_cards::formats::check(baylee_cards::formats::Shape::Constructed, &main, &[])
                .is_empty()
        );
        let mut counts = BTreeMap::new();
        for card in main {
            *counts.entry(card).or_insert(0_i32) += 1;
        }
        counts
    };
    for counts in api::coordinates() {
        let before = cards(counts);
        if counts.iter().sum::<u8>() == 4 {
            continue;
        }
        for track in api::Track::ALL {
            let mut next = counts;
            next[track.index()] += 1;
            let after = cards(next);
            let offer = &theme.upgrades[track.index()][usize::from(counts[track.index()])];
            let remove = baylee_cards::decks::by_name(&offer.remove).unwrap();
            let add = baylee_cards::decks::by_name(&offer.add).unwrap();
            let mut expected = before.clone();
            *expected.get_mut(&remove).unwrap() -= i32::from(offer.copies);
            *expected.entry(add).or_default() += i32::from(offer.copies);
            expected.retain(|_, n| *n != 0);
            assert_eq!(after, expected);
        }
    }
    assert!(deck::player(&theme, [5, 0, 0], &generator.working).is_err());
}

#[test]
fn tower_generation_refuses_invalid_bounds_before_playing() {
    let generator = Generator::from_source(&crate::working::repo_root()).unwrap();
    let period = Period::containing(Kind::Weekly, 1_794_182_400).unwrap();
    assert!(generator.tower(period, 41, 0, Limits::default()).is_err());
    let daily = Period::containing(Kind::Daily, period.starts_at).unwrap();
    assert!(generator.tower(daily, 41, 1, Limits::default()).is_err());
}

#[test]
fn supplied_opening_uses_exactly_seven_cards_from_the_legal_deck() {
    let generator = Generator::from_source(&crate::working::repo_root()).unwrap();
    let period = Period::containing(Kind::Weekly, 1_794_182_400).unwrap();
    let theme = deck::theme(period);
    let pool = deck::opponent_pool(&generator.working).unwrap();
    for counts in api::coordinates() {
        let (preset, text) = deck::build(&theme, counts, 41, &pool, &generator.working).unwrap();
        let seat = &preset.seats[0];
        assert_eq!(seat.deck.len(), 53);
        assert_eq!(seat.starting_hand.as_ref().unwrap().len(), 7);
        let mut actual = seat
            .deck
            .iter()
            .chain(seat.starting_hand.iter().flatten())
            .map(|entry| entry.card)
            .collect::<Vec<_>>();
        let mut expected = baylee_cards::decks::house_deck(&text)
            .unwrap()
            .main
            .into_iter()
            .map(|card| card.index)
            .collect::<Vec<_>>();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
        assert!((60..=75).contains(&preset.seats[1].deck.len()));
    }
}

#[test]
fn human_deal_search_preserves_the_opponent_opening_and_engine_random_stream() {
    use baylee_core::ids::PlayerId;
    use baylee_engine::{state::GameState, zone::ZoneLocation};
    let generator = Generator::from_source(&crate::working::repo_root()).unwrap();
    let theme = deck::theme(Period::containing(Kind::Weekly, 1_794_182_400).unwrap());
    let pool = deck::opponent_pool(&generator.working).unwrap();
    let cards = |state: &GameState, zone| {
        state
            .zones
            .list(zone)
            .iter()
            .map(|&id| state.object(id).unwrap().card)
            .collect::<Vec<_>>()
    };
    for counts in api::coordinates() {
        let (preset, _) = deck::build(&theme, counts, 41, &pool, &generator.working).unwrap();
        let base = GameState::from_preset(&preset, &baylee_gamehost::RegistryLookup).unwrap();
        for deal in 1..encounter::DEALS {
            let mut arranged = preset.clone();
            encounter::arrange(&mut arranged, counts, deal);
            let mut again = preset.clone();
            encounter::arrange(&mut again, counts, deal);
            assert_eq!(arranged, again);
            assert_eq!(arranged.seats[1], preset.seats[1]);
            assert_eq!(arranged.house_rules, preset.house_rules);
            assert_eq!(
                arranged.seats[0].starting_hand,
                preset.seats[0].starting_hand
            );
            let mut before = preset.seats[0]
                .deck
                .iter()
                .map(|e| e.card)
                .collect::<Vec<_>>();
            let mut after = arranged.seats[0]
                .deck
                .iter()
                .map(|e| e.card)
                .collect::<Vec<_>>();
            before.sort_unstable();
            after.sort_unstable();
            assert_eq!(before, after);
            let state =
                GameState::from_preset(&arranged, &baylee_gamehost::RegistryLookup).unwrap();
            for zone in [
                ZoneLocation::Library(PlayerId::new(1)),
                ZoneLocation::Hand(PlayerId::new(1)),
            ] {
                assert_eq!(cards(&state, zone), cards(&base, zone));
            }
            assert_eq!(state.rng.seed(), base.rng.seed());
            assert_eq!(state.rng.word_pos(), base.rng.word_pos());
        }
    }
}
