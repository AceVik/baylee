//! Child of `transforms_remaining`. Actual Brass resolution with oversized hand.
#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn brass_paged_discard_accumulates_all_pages_before_discarding_and_draws_full_count() {
    for select_all in [false, true] {
        // Real card definitions unchanged; extra ordinary Forests provide the
        // large hand and a sufficiently large library without drawing empty.
        let mut opening = vec![index::FOREST; 900];
        opening.push(index::BRASS_S_TUNNEL_GRINDER);
        let mut e = prepared(&opening, &[], &[]);
        let forests: Vec<_> = e
            .state
            .zones
            .list(ZoneLocation::Hand(P0))
            .iter()
            .copied()
            .filter(|id| {
                e.state
                    .object(*id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == index::FOREST)
            })
            .collect();
        for id in forests.iter().take(600) {
            e.state
                .move_object(
                    *id,
                    ZoneLocation::Library(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        let hand_before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
        let library_before = library_size(&e, P0);
        let graveyard_before = e.state.zones.list(ZoneLocation::Graveyard(P0)).len();
        cast(&mut e, index::BRASS_S_TUNNEL_GRINDER);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        assert_page_selections_change_continuation_hash(&mut e);
        let mut selected = Vec::new();
        let mut offered = Vec::new();
        let mut pages = 0;
        while let Pending::ChooseCards { options, .. } = e.pending().clone() {
            assert!(options.len() <= 255, "agreed paged selection interface");
            assert!(!options.is_empty());
            for id in &options {
                assert!(
                    !offered.contains(id),
                    "each hand card appears on only one page"
                );
            }
            offered.extend_from_slice(&options);
            assert_eq!(
                e.state.zones.list(ZoneLocation::Graveyard(P0)).len(),
                graveyard_before,
                "earlier page selections must not discard prematurely"
            );
            assert_eq!(
                library_size(&e, P0),
                library_before,
                "no draw until all pages answered"
            );
            let page = if select_all { options } else { vec![] };
            selected.extend_from_slice(&page);
            choose(&mut e, page);
            pages += 1;
            assert!(pages <= 4, "paged selection must make progress");
        }
        assert!(pages >= 2);
        assert_eq!(offered.len(), hand_before - 1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(selected.len(), if select_all { hand_before - 1 } else { 0 });
        assert_eq!(
            e.state.zones.list(ZoneLocation::Graveyard(P0)).len(),
            graveyard_before + selected.len()
        );
        assert_eq!(library_size(&e, P0), library_before - selected.len() - 1);
        assert_eq!(
            e.state.zones.list(ZoneLocation::Hand(P0)).len(),
            hand_before
        );
    }
}

fn assert_page_selections_change_continuation_hash(e: &mut Engine<RegistryLookup>) {
    // Engine has no Clone API. Its complete decision checkpoint provides
    // the same exact first-page branch point, including the continuation.
    let checkpoint = crate::engine::checkpoint::Checkpoint::capture(e);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let selected_card = options[0];
    let restart = crate::engine::checkpoint::Checkpoint::capture(e);
    let untouched_state = e.state.snapshot_hash();
    choose(e, vec![]);
    let empty_engine_hash = e.snapshot_hash();
    let empty_next_prompt = format!("{:?}", e.pending());
    assert_eq!(e.state.snapshot_hash(), untouched_state);
    checkpoint.restore(e);
    choose(e, vec![selected_card]);
    assert_eq!(
        e.state.snapshot_hash(),
        untouched_state,
        "a page answer records intentions without touching game state"
    );
    assert_eq!(
        format!("{:?}", e.pending()),
        empty_next_prompt,
        "the next page is independent of the prior page selection"
    );
    assert_ne!(
        e.snapshot_hash(),
        empty_engine_hash,
        "previous-page selections are semantically different continuations"
    );
    // Restore first page so the original full-count assertions remain intact.
    restart.restore(e);
}
