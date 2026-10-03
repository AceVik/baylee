use super::*;

#[test]
fn controlled_choice_projection_keeps_private_zones_separate_and_revokes_access() {
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    let state = GameState::from_preset(&preset, &Registry).unwrap();
    let (actor, subject, third) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let hand = state.zones.list(ZoneLocation::Hand(subject)).clone();
    assert!(!hand.is_empty());
    let pending = Pending::ChooseCards {
        player: subject,
        options: hand.clone(),
        min: 0,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown {
            x: 2,
            paid: [0, 0, 0, 0, 0, 2],
            fixed_cost: ManaCost::ZERO,
        },
        total: None,
    };
    for viewer in [actor, subject, third] {
        let mut controlled = SeatSet::new();
        if viewer == actor {
            controlled.insert(subject);
        }
        let ctx = SeatContext {
            awaiting: Some(actor),
            decision_player: Some(subject),
            controlled_players: controlled,
            ..SeatContext::default()
        };
        // Even an accidentally unfiltered choice cannot reveal its options
        // to an unrelated viewer; both legitimate participants retain them.
        let shown = player_view(&state, viewer, 0, Some(&pending), &ctx, &[]);
        if viewer == third {
            assert!(shown.looking_at.is_empty());
            assert!(shown.controlled_hands.is_empty());
        } else {
            assert_eq!(
                shown
                    .looking_at
                    .iter()
                    .map(|object| object.id)
                    .collect::<Vec<_>>(),
                hand
            );
            assert!(shown.looking_at.iter().all(|object| object.card.is_some()));
        }
        if viewer == actor {
            assert_eq!(shown.controlled_hands.len(), 1);
            assert_eq!(shown.controlled_hands[0].player, subject);
            assert_eq!(
                shown.controlled_hands[0]
                    .cards
                    .iter()
                    .map(|object| object.id)
                    .collect::<Vec<_>>(),
                hand
            );
            assert!(shown.hand.iter().all(|object| !hand.contains(&object.id)));
            let decoded: PlayerView =
                serde_json::from_str(&serde_json::to_string(&shown).unwrap()).unwrap();
            assert_eq!(decoded.controlled_hands, shown.controlled_hands);
        }
    }
    let revoked = player_view(&state, actor, 1, None, &SeatContext::default(), &[]);
    assert!(revoked.looking_at.is_empty());
    assert!(revoked.controlled_hands.is_empty());
}

#[test]
fn wide_public_mana_counts_keep_each_type_and_the_full_total() {
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    for color in baylee_core::mana::ManaColor::ALL {
        state.players[0].mana_pool.add(color, u32::MAX);
    }
    let view = player_view(
        &state,
        PlayerId::new(0),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    assert_eq!(view.seats[0].mana_pool.white, u32::MAX);
    assert_eq!(view.seats[0].mana_pool.colorless, u32::MAX);
    assert_eq!(view.seats[0].mana_pool.total(), 6 * u64::from(u32::MAX));
    let decoded: PlayerView = serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
    assert_eq!(decoded.seats[0].mana_pool, view.seats[0].mana_pool);
}
