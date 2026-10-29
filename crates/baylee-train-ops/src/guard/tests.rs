use super::*;

#[test]
fn lost_wait_ownership_never_signals_a_recycled_process_group() {
    let mut child = Command::new("true").process_group(0).spawn().unwrap();
    let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
    child.wait().unwrap();
    let mut group = OwnedGroup {
        child,
        pid,
        reaped: true,
    };
    assert!(group.signal(Signal::TERM).is_err());
    group.reaped = false;
    assert!(group.signal(Signal::TERM).is_err());
    group.reaped = true;
}
