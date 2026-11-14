use super::*;
use baylee_engine::{choice::PriorityHold, engine::DetectLoops};
use baylee_protocol::v1::{SeatMind, seat_mind::Kind};

fn rebuild(
    preset: &GamePreset,
    bytes: &[u8],
) -> Result<adjudicate::Rebuilt<DetectLoops, crate::RegistryLookup>, Refused> {
    adjudicate::rebuild(preset, None, bytes, budget::Limits::legacy(), deadline())
}

#[test]
fn recovered_limits_count_real_settings_and_not_automatic_undeclarations() {
    let (preset, _) = fixture(false);
    let mut live = Session::new_recorded(&preset, record::this_build()).unwrap();
    live.tell_time(12_345);
    live.declare_mind(
        ME,
        &SeatMind {
            kind: Kind::Human.into(),
            ..SeatMind::default()
        },
    )
    .unwrap();
    let hold = PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn: 0 });
    let clear = PlayerAction::SetPriorityHold(PriorityHold::Always);
    live.act_slice(ME, hold.clone(), 0).unwrap();
    live.socket_opened(ME);
    live.act_slice(ME, clear, 0).unwrap();
    live.declare_mind(
        ME,
        &SeatMind {
            kind: Kind::Scripted.into(),
            ..SeatMind::default()
        },
    )
    .unwrap();
    live.act_slice(ME, hold, 0).unwrap();
    let bytes = live.take_record();
    let restored = rebuild(&preset, &bytes).unwrap();
    assert_eq!(restored.session.settings_made, live.settings_made);
    assert_eq!(restored.session.settings_made[0], 3);
    assert_eq!(restored.session.declarations, live.declarations);
    assert_eq!(restored.session.declarations[0], 2);
    assert_eq!(restored.session.minds, live.minds);
    assert_eq!(restored.at, 12_345);
    assert_eq!(restored.session.snapshot_hash(), live.snapshot_hash());
    assert!(!restored.ended);
}

#[test]
fn restoration_refuses_missing_terminal_metadata_and_decreasing_timestamps() {
    let (preset, bytes) = fixture(false);
    let missing_end = rewrite(&bytes, |lines| {
        lines.pop();
    });
    assert!(matches!(
        rebuild(&preset, &missing_end),
        Err(Refused::Invalid)
    ));
    let backwards = rewrite(&bytes, |lines| {
        let mut count = 0;
        for line in lines {
            if let Line::Input { at, .. } = line {
                *at = if count == 0 { 10 } else { 9 };
                count += 1;
            }
        }
        assert!(count > 1);
    });
    assert!(matches!(
        rebuild(&preset, &backwards),
        Err(Refused::Invalid)
    ));
}

#[test]
fn declared_metadata_is_validated_before_it_is_restored() {
    let (preset, _) = fixture(false);
    let mut session = Session::new(&preset).unwrap();
    assert_eq!(
        metadata::declaration(&mut session, record::Mind::undeclared()),
        Err(Refused::Invalid)
    );
    let mind = record::Mind::from_wire(&SeatMind {
        kind: Kind::Human.into(),
        ..SeatMind::default()
    })
    .unwrap();
    metadata::declaration(&mut session, mind.clone()).unwrap();
    assert_eq!(
        metadata::declaration(&mut session, mind.clone()),
        Err(Refused::Invalid)
    );
    let mut wrong = mind;
    wrong.model = "invalid://private-payload".into();
    assert_eq!(
        metadata::declaration(&mut session, wrong),
        Err(Refused::Invalid)
    );
    metadata::declaration(&mut session, record::Mind::undeclared()).unwrap();
    assert_eq!(session.declarations[0], 1);
}
