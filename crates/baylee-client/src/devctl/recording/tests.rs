use super::writer::write_or_cancel;
use super::*;
use std::io::Write;

fn running() -> Recorder {
    let mut recorder = Recorder::default();
    start(&mut recorder, r#"{"duration_ms":1000,"fps":10}"#).unwrap();
    let run = recorder.run.as_mut().unwrap();
    assert!(matches!(
        run.events
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap(),
        Event::Ready
    ));
    run.start = Some(Instant::now());
    run.summary.phase = "capturing";
    recorder
}

#[test]
fn two_readbacks_hold_capacity_until_drained_and_cancel_stops_scheduling() {
    let mut recorder = running();
    let world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let window = Window::default();
    let clock = Time::<Virtual>::default();
    let run = recorder.run.as_mut().unwrap();
    let directory = run.summary.directory.clone();
    for index in 0..3 {
        run.start = Some(
            Instant::now()
                .checked_sub(Duration::from_millis(index * 100))
                .unwrap(),
        );
        schedule(run, &mut commands, index, &clock, Some(&window), false);
    }
    assert_eq!(run.pending.len(), 2);
    assert_eq!(run.summary.requested, 2);
    assert_eq!(run.summary.dropped, 1);
    assert_eq!(run.summary.gaps[0].reason, "backpressure");
    assert!(start(&mut recorder, r#"{"duration_ms":1000,"fps":10}"#).is_err());
    stop(&mut recorder, r#"{"cancel":true}"#).unwrap();
    let run = recorder.run.as_mut().unwrap();
    assert_eq!(run.summary.phase, "draining");
    assert!(run.cancelled.load(Ordering::Relaxed));
    run.finish();
    assert!(!run.final_sent, "pending readbacks must keep their permits");
    drop(recorder);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn worker_writes_png_before_ledger_and_cancel_preserves_written_frames() {
    let mut recorder = running();
    let run = recorder.run.as_mut().unwrap();
    let directory = run.summary.directory.clone();
    for slot in 0..2 {
        let metadata = Sample {
            slot,
            file: format!("frame-{slot:06}.png"),
            nominal_us: 0,
            requested_us: 0,
            request_frame: 0,
            started_unix_ms: 0,
            virtual_us: 0,
            virtual_speed: 1.0,
            virtual_paused: false,
            received_us: 0,
            received_frame: 0,
            width: 2,
            height: 2,
        };
        let image = Image::from_dynamic(image::DynamicImage::new_rgba8(2, 2), true, default());
        run.writer
            .try_send(Work::Frame(metadata, Box::new(image)))
            .unwrap();
        let event = run
            .events
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert!(
            matches!(event, Event::Written(index, Ok(written)) if index == slot && written == (slot == 0))
        );
        run.cancelled.store(true, Ordering::Relaxed);
    }
    let png = image::open(directory.join("frame-000000.png")).unwrap();
    assert_eq!((png.width(), png.height()), (2, 2));
    assert!(!directory.join("frame-000001.png").exists());
    let ledger = std::fs::read_to_string(directory.join("frames.jsonl")).unwrap();
    assert_eq!(ledger.lines().count(), 1);
    let row: serde_json::Value = serde_json::from_str(ledger.trim()).unwrap();
    assert_eq!(row["file"], "frame-000000.png");
    run.writer
        .try_send(Work::Finish(r#"{"phase":"cancelled"}"#.into()))
        .unwrap();
    assert!(matches!(
        run.events
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap(),
        Event::Finished(Ok(()))
    ));
    assert_eq!(
        std::fs::read_to_string(directory.join("recording.json")).unwrap(),
        r#"{"phase":"cancelled"}"#
    );
    drop(recorder);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn stop_before_ready_never_reopens_the_clock_or_changes_final_metadata() {
    for cancel in [false, true] {
        let mut recorder = Recorder::default();
        start(&mut recorder, r#"{"duration_ms":1000,"fps":10}"#).unwrap();
        stop(&mut recorder, &format!("{{\"cancel\":{cancel}}}")).unwrap();
        let run = recorder.run.as_mut().unwrap();
        let directory = run.summary.directory.clone();
        run.finish();
        let deadline = Instant::now() + Duration::from_secs(5);
        while run.summary.phase == "draining" {
            assert!(Instant::now() < deadline, "worker did not finish");
            run.receive();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(run.start.is_none());
        assert_eq!(run.summary.started_unix_ms, 0);
        assert_eq!(run.summary.capture_end_us, Some(0));
        assert_eq!(
            run.summary.phase,
            if cancel { "cancelled" } else { "complete" }
        );
        let saved: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(directory.join("recording.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved["started_unix_ms"], 0);
        assert_eq!(saved["capture_end_us"], 0);
        assert_eq!(saved["phase"], run.summary.phase);
        drop(recorder);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn requests_are_strict_and_bounded() {
    for body in [
        r#"{"duration_ms":0,"fps":10}"#,
        r#"{"duration_ms":30001,"fps":1}"#,
        r#"{"duration_ms":1,"fps":61}"#,
        r#"{"duration_ms":30000,"fps":60}"#,
        r#"{"duration_ms":1000,"fps":10,"path":"/tmp/other"}"#,
        r#"{"duration_ms":1,"fps":NaN}"#,
    ] {
        assert!(Options::parse(body).is_err(), "{body}");
    }
    assert_eq!(
        Options::parse(r#"{"duration_ms":15000,"fps":60}"#)
            .unwrap()
            .samples(),
        900
    );
}

#[test]
fn late_frames_skip_instead_of_bursting() {
    assert_eq!(due_slots(0, 10, 0, 10), (Some(0), 0));
    assert_eq!(due_slots(50_000, 10, 1, 10), (None, 0));
    assert_eq!(due_slots(350_000, 10, 1, 10), (Some(3), 2));
    assert_eq!(due_slots(5_000_000, 10, 1, 10), (Some(9), 8));
    assert_eq!(due_slots(5_000_000, 10, 10, 10), (None, 0));
}

#[test]
fn http_rejects_browser_rebinding_wrong_verbs_and_large_heads() {
    let host = ("host".into(), "127.0.0.1:28770".into());
    assert!(validate_http("POST", "/recording/start", std::slice::from_ref(&host)).is_ok());
    assert!(validate_http("GET", "/recording/start", std::slice::from_ref(&host)).is_err());
    assert!(
        validate_http(
            "POST",
            "/recording/start",
            &[host, ("origin".into(), "null".into())]
        )
        .is_err()
    );
    assert!(
        validate_http(
            "POST",
            "/recording/start",
            &[("host".into(), "evil.test".into())]
        )
        .is_err()
    );
    let mut input = std::io::Cursor::new(b"123456789\n");
    assert!(header_line(&mut input, &mut 8).is_err());
}

fn headless() -> (App, PathBuf) {
    let mut app = App::new();
    let (_, jobs) = channel();
    app.insert_resource(DevControl {
        jobs: Mutex::new(jobs),
        held: Vec::new(),
        clicking: Vec::new(),
        stepping: None,
        shell_jobs: Vec::new(),
        reveal_jobs: Vec::new(),
        frame: 0,
        virtual_cursor: None,
        served: 0,
        idle_exit: None,
    })
    .init_resource::<Time<Virtual>>()
    .insert_resource(running());
    install(&mut app);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let directory = app
        .world()
        .resource::<Recorder>()
        .run
        .as_ref()
        .unwrap()
        .summary
        .directory
        .clone();
    (app, directory)
}

fn pending_entity(app: &App) -> Entity {
    let run = app.world().resource::<Recorder>().run.as_ref().unwrap();
    let Stage::Readback { entity, .. } = run.pending.values().next().unwrap().stage else {
        panic!("expected readback");
    };
    entity
}

#[test]
fn late_deferred_primary_captures_win_but_another_window_does_not_conflict() {
    // The producer runs in Last, so this also fails if the recorder is placed
    // in an unordered Last system before that schedule applies its Commands.
    for target in 0..3 {
        let (mut app, directory) = headless();
        let other = app.world_mut().spawn(Window::default()).id();
        app.add_systems(
            Last,
            move |mut commands: Commands, primary: Query<Entity, With<PrimaryWindow>>| {
                let shot = match target {
                    0 => Screenshot::primary_window(),
                    1 => Screenshot::window(primary.single().unwrap()),
                    _ => Screenshot::window(other),
                };
                commands.spawn(shot);
            },
        );
        app.update();
        let run = app.world().resource::<Recorder>().run.as_ref().unwrap();
        if target < 2 {
            assert!(run.pending.is_empty());
            assert_eq!(run.summary.requested, 0);
            assert_eq!(run.summary.gaps[0].reason, "target_busy");
        } else {
            assert_eq!(run.pending.len(), 1);
            assert_eq!(run.summary.requested, 1);
        }
        drop(app);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn lost_callback_recovers_only_a_never_submitted_or_delivered_readback() {
    for (was_submitted, was_delivered) in [(false, false), (true, true), (true, false)] {
        let (mut app, directory) = headless();
        app.update();
        let entity = pending_entity(&app);
        if was_submitted {
            app.world_mut().entity_mut(entity).insert(Capturing);
        }
        if was_delivered {
            app.world_mut().entity_mut(entity).insert(Captured);
        }
        // Duplicate rejection has no Capturing; normal cleanup follows
        // Captured. Losing an actually in-flight entity must NOT free memory.
        app.world_mut().despawn(entity);
        stop(&mut app.world_mut().resource_mut::<Recorder>(), "{}").unwrap();
        app.update();
        let recorder = app.world().resource::<Recorder>();
        let run = recorder.run.as_ref().unwrap();
        if was_submitted && !was_delivered {
            assert_eq!(run.pending.len(), 1);
            assert!(!run.final_sent);
        } else {
            assert!(run.pending.is_empty());
            assert_eq!(run.summary.gaps.last().unwrap().reason, "callback_lost");
            assert!(run.final_sent);
            assert!(matches!(
                run.events
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap(),
                Event::Finished(Ok(()))
            ));
        }
        drop(app);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn late_old_run_callback_cannot_consume_the_new_runs_same_slot() {
    let (mut app, directory) = headless();
    let old_id = uuid::Uuid::now_v7();
    let metadata = Sample {
        slot: 0,
        file: "frame-000000.png".into(),
        nominal_us: 0,
        requested_us: 0,
        request_frame: 0,
        started_unix_ms: 0,
        virtual_us: 0,
        virtual_speed: 1.0,
        virtual_paused: false,
        received_us: 0,
        received_frame: 0,
        width: 2,
        height: 2,
    };
    let entity = app
        .world_mut()
        .spawn_empty()
        .observe(captured(old_id, metadata))
        .id();
    app.world_mut()
        .resource_mut::<Recorder>()
        .run
        .as_mut()
        .unwrap()
        .pending
        .insert(
            0,
            Pending {
                requested: Instant::now(),
                stage: Stage::Readback {
                    entity,
                    submitted: false,
                    delivered: false,
                },
            },
        );
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: Image::from_dynamic(image::DynamicImage::new_rgba8(2, 2), true, default()),
    });
    let run = app.world().resource::<Recorder>().run.as_ref().unwrap();
    assert_ne!(run.summary.id, old_id);
    assert!(matches!(run.pending[&0].stage, Stage::Readback { .. }));
    assert_eq!(run.summary.written, 0);
    assert_eq!(
        std::fs::metadata(directory.join("frames.jsonl"))
            .unwrap()
            .len(),
        0
    );
    drop(app);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn partial_ledger_write_poison_prevents_later_queued_appends() {
    let mut ledger = b"{\"slot\":0}\n".to_vec();
    let mut error = None;
    let cancelled = AtomicBool::new(false);
    let first = write_or_cancel(&mut error, &cancelled, || {
        ledger.write_all(b"{\"slot\":").unwrap();
        Err("injected short write / disk full".into())
    });
    assert!(first.is_err());
    let prefix = ledger.clone();
    let second = write_or_cancel(&mut error, &cancelled, || {
        ledger.write_all(b"{\"slot\":2}\n").unwrap();
        Ok(())
    });
    assert!(second.is_err());
    assert_eq!(ledger, prefix, "nothing may follow a failed partial append");
}

#[test]
fn video_option_writes_mp4_and_step_mp4_on_finish() {
    let mut recorder = Recorder::default();
    start(
        &mut recorder,
        r#"{"duration_ms":1000,"fps":10,"video":true}"#,
    )
    .unwrap();
    let run = recorder.run.as_mut().unwrap();
    assert!(matches!(
        run.events
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap(),
        Event::Ready
    ));
    run.start = Some(Instant::now());
    run.summary.phase = "capturing";
    let directory = run.summary.directory.clone();
    for slot in 0..2 {
        let metadata = Sample {
            slot,
            file: format!("frame-{slot:06}.png"),
            nominal_us: u64::from(slot) * 100_000,
            requested_us: u64::from(slot) * 100_000,
            request_frame: u64::from(slot),
            started_unix_ms: 0,
            virtual_us: u128::from(slot) * 100_000,
            virtual_speed: 1.0,
            virtual_paused: false,
            received_us: u64::from(slot) * 100_000,
            received_frame: u64::from(slot),
            width: 32,
            height: 24,
        };
        let image = Image::from_dynamic(image::DynamicImage::new_rgba8(32, 24), true, default());
        run.writer
            .try_send(Work::Frame(metadata, Box::new(image)))
            .unwrap();
    }
    for _ in 0..2 {
        run.receive();
        while run.summary.written < 2 {
            std::thread::sleep(Duration::from_millis(5));
            run.receive();
        }
    }
    run.end("duration");
    run.finish();
    let deadline = Instant::now() + Duration::from_secs(5);
    while run.summary.phase == "draining" {
        assert!(Instant::now() < deadline, "worker did not finish");
        run.receive();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(run.summary.phase, "complete");
    for name in ["recording.mp4", "recording-step.mp4"] {
        let bytes = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(&bytes[4..8], b"ftyp", "{name}");
        let ftyp_len = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let mdat_len =
            u32::from_be_bytes(bytes[ftyp_len..ftyp_len + 4].try_into().unwrap()) as usize;
        let moov_pos = ftyp_len + mdat_len;
        assert_eq!(&bytes[moov_pos + 4..moov_pos + 8], b"moov", "{name}");
        // mvhd starts at moov_pos + 8; version 0 timescale is at +28..+32 and duration at +32..+36.
        let mvhd_pos = moov_pos + 8;
        assert_eq!(&bytes[mvhd_pos + 4..mvhd_pos + 8], b"mvhd", "{name}");
        let duration_ms =
            u32::from_be_bytes(bytes[mvhd_pos + 24..mvhd_pos + 28].try_into().unwrap());
        assert!(
            duration_ms >= 1000,
            "{name} duration_ms={duration_ms} must be >= 1000 for 1 FPS video tokenizers"
        );
    }
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(directory.join("recording.json")).unwrap())
            .unwrap();
    assert!(saved["video"].as_str().unwrap().ends_with("recording.mp4"));
    assert!(
        saved["video_step"]
            .as_str()
            .unwrap()
            .ends_with("recording-step.mp4")
    );
    drop(recorder);
    std::fs::remove_dir_all(directory).unwrap();
}
