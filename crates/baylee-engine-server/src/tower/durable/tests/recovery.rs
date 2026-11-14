use super::*;
use baylee_gamehost::session::tower::ReplayBudget;

fn open(table: &mut Table, record: &mut Vec<u8>, digest: [u8; 32]) {
    table
        .handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                    seat: 0,
                    resync: false,
                })),
            },
            &[],
        )
        .unwrap();
    commit(table, record, digest);
    table
        .handle(seat(v1::envelope::Msg::SeatReady(v1::SeatReady {})), &[])
        .unwrap();
    commit(table, record, digest);
    table.tell_time(table.entrance_deadline().unwrap()).unwrap();
    table.finish_entrance().unwrap();
    commit(table, record, digest);
}

fn next(table: &mut Table) {
    if table.needs_pump() {
        table.pump(&[]).unwrap();
    } else {
        let action = table
            .driver
            .session()
            .unwrap()
            .house_action(PlayerId::new(0))
            .unwrap();
        play(table, &action);
    }
}

fn recover(
    package: &Package,
    catalog: &Catalog,
    prefix: Prefix,
    source: &[u8],
) -> Result<Table, Refusal> {
    Table::resume(
        "durable-test".into(),
        package,
        catalog,
        ["Player returned".into(), "Echo returned".into()],
        Recovery {
            binding: Binding {
                package: package.digest(catalog).unwrap(),
                fence: 9,
                prefix,
            },
            byte_limit: MAX_RECORD_BYTES,
            source,
            resources: ReplayBudget::default(),
            deadline: Instant::now() + Duration::from_secs(10),
        },
    )
}

fn fresh(package: &Package, catalog: &Catalog) -> (Table, Vec<u8>) {
    let mut table = Table::new(
        "durable-test".into(),
        package,
        catalog,
        4,
        MAX_RECORD_BYTES,
        ["Player".into(), "Echo".into()],
    )
    .unwrap();
    let mut record = Vec::new();
    let digest = package.digest(catalog).unwrap();
    commit(&mut table, &mut record, digest);
    open(&mut table, &mut record, digest);
    (table, record)
}

#[test]
fn recovery_at_human_and_expert_boundaries_preserves_future_record_bytes() {
    let (package, catalog) = fixture();
    let digest = package.digest(&catalog).unwrap();
    let (mut live, mut original) = fresh(&package, &catalog);
    let (mut saw_human, mut saw_expert) = (false, false);
    for expert_boundary in [false, true, false, true] {
        for _ in 0..128 {
            if live.needs_pump() == expert_boundary {
                break;
            }
            next(&mut live);
            commit(&mut live, &mut original, digest);
        }
        assert_eq!(live.needs_pump(), expert_boundary);
        saw_expert |= live.needs_pump();
        saw_human |= !live.needs_pump();
        let prefix = live.acknowledged();
        let hash = live.driver.session().unwrap().snapshot_hash();
        let mut restored = recover(&package, &catalog, prefix, &original).unwrap();
        assert_eq!(restored.acknowledged(), prefix);
        assert_eq!(restored.driver.session().unwrap().snapshot_hash(), hash);
        assert_eq!(
            restored.driver.session().unwrap().seq(),
            live.driver.session().unwrap().seq()
        );
        assert!(restored.batch().is_none(), "no repeated header or inputs");
        assert!(!restored.needs_pump(), "fresh loading handshake first");
        assert_eq!(restored.acknowledge(4, prefix), Err(Refusal::Stale));
        assert!(restored.acknowledge(9, prefix).unwrap().is_empty());
        let mut resumed_record = original.clone();
        // Both hosts receive the same time. Opening the returning presentation
        // may perform one Expert action, just like the uninterrupted table pump.
        let at = live.driver.now + 20_000;
        restored.tell_time(at).unwrap();
        open(&mut restored, &mut resumed_record, digest);
        live.tell_time(restored.driver.now).unwrap();
        if live.needs_pump() {
            live.pump(&[]).unwrap();
            commit(&mut live, &mut original, digest);
        }
        assert_eq!(resumed_record, original);
        restored.tell_time(0).unwrap();
        assert_eq!(
            restored.driver.now, live.driver.now,
            "clock rollback is clamped"
        );
        for _ in 0..24 {
            next(&mut live);
            next(&mut restored);
            commit(&mut live, &mut original, digest);
            commit(&mut restored, &mut resumed_record, digest);
            assert_eq!(
                resumed_record, original,
                "Expert memory/RNG and line numbering survive recovery"
            );
            assert_eq!(
                restored.driver.session().unwrap().decision_seq(),
                live.driver.session().unwrap().decision_seq()
            );
        }
    }
    assert!(saw_human && saw_expert, "exercise both decision boundaries");
}

#[test]
fn recovery_preserves_paid_potions_and_terminal_commit_before_seal() {
    let (mut package, catalog) = fixture();
    package.builds[0].potions = [1, 1];
    let digest = package.digest(&catalog).unwrap();
    let (mut live, mut original) = fresh(&package, &catalog);
    let me = PlayerId::new(0);
    for _ in 0..32 {
        if !live.needs_pump()
            && matches!(live.driver.session().unwrap().pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        }
        next(&mut live);
        commit(&mut live, &mut original, digest);
    }
    let potion =
        baylee_cards::tokens::tower::definition(baylee_core::preset::TowerSupply::HealingPotion);
    let session = live.driver.session().unwrap();
    let source = *session
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Command(me))
        .iter()
        .find(|id| {
            session
                .state()
                .object(**id)
                .and_then(|object| object.token)
                .is_some_and(|token| std::ptr::eq(token, potion))
        })
        .unwrap();
    play(
        &mut live,
        &PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    );
    commit(&mut live, &mut original, digest);
    let mut restored = recover(&package, &catalog, live.acknowledged(), &original).unwrap();
    let mut record = original;
    open(&mut restored, &mut record, digest);
    for _ in 0..32 {
        if !restored.needs_pump() {
            break;
        }
        restored.pump(&[]).unwrap();
        commit(&mut restored, &mut record, digest);
    }
    play(&mut restored, &PlayerAction::Concede);
    commit(&mut restored, &mut record, digest);
    assert!(restored.finished());
    let final_prefix = restored.acknowledged();
    let terminal = recover(&package, &catalog, final_prefix, &record).unwrap();
    assert!(
        terminal.finished(),
        "committed ending needs sealing, not another move"
    );
    assert!(terminal.batch().is_none());
    let settlement = super::super::super::verification::verify_record(
        &package,
        &catalog,
        Binding {
            package: digest,
            fence: 9,
            prefix: final_prefix,
        },
        record.as_slice(),
        ReplayBudget::default(),
        Instant::now() + Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(settlement.used, [1, 0]);
    assert_eq!(settlement.outcome, Outcome::Defeat);
}

#[test]
fn recovery_refuses_altered_truncated_overlong_and_uncommitted_streams() {
    let (package, catalog) = fixture();
    let (live, original) = fresh(&package, &catalog);
    let prefix = live.acknowledged();
    let mut altered = original.clone();
    *altered.last_mut().unwrap() ^= 1;
    let mut excess = original.clone();
    excess.push(0);
    for bad in [&altered[..], &original[..original.len() - 1], &excess, &[]] {
        assert!(matches!(
            recover(&package, &catalog, prefix, bad),
            Err(Refusal::Invalid)
        ));
    }
    let mut other_prefix = prefix;
    other_prefix.digest[0] ^= 1;
    assert!(matches!(
        recover(&package, &catalog, other_prefix, &original),
        Err(Refusal::Invalid)
    ));
}

#[test]
fn native_verification_accepts_legal_concession_before_expert_opening() {
    let (package, catalog) = fixture();
    let identity = crate::variant::identity();
    let preset = package
        .preset(&catalog, &identity, |card| {
            Some(baylee_cards::decks::reference_print(card))
        })
        .unwrap();
    let mut session =
        crate::variant::HostedSession::recorded_for_execution(&preset, &identity).unwrap();
    assert!(session.needs_pump());
    session
        .act_slice(PlayerId::new(0), PlayerAction::Concede, 0)
        .unwrap();
    let digest = package.digest(&catalog).unwrap();
    let mut prefix = Prefix::default();
    let mut source = Vec::new();
    for chunk in session.take_record().chunks(MAX_CHUNK_BYTES) {
        prefix = prefix.extended(digest, MAX_RECORD_BYTES, chunk).unwrap();
        baylee_worker::frame::write(&mut source, chunk, MAX_CHUNK_BYTES).unwrap();
    }
    let settlement = super::super::super::verification::verify_record(
        &package,
        &catalog,
        Binding {
            package: digest,
            fence: 1,
            prefix,
        },
        source.as_slice(),
        ReplayBudget::default(),
        Instant::now() + Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(settlement.outcome, Outcome::Defeat);
    assert_eq!(settlement.turns, 1);
}
