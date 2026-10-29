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

#[test]
fn invocation_preserves_symlinks_for_absolute_relative_and_path_search() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().canonicalize().unwrap();
    let bin = cwd.join("runtime/bin");
    fs::create_dir_all(&bin).unwrap();
    let target = std::env::current_exe().unwrap().canonicalize().unwrap();
    let alias = bin.join("worker");
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    for (command, path) in [
        (alias.to_str().unwrap(), ""),
        ("runtime/bin/worker", ""),
        ("worker", "runtime/bin"),
        ("worker", bin.to_str().unwrap()),
    ] {
        let resolved = executable_in_path(command, &cwd, std::ffi::OsStr::new(path)).unwrap();
        assert_eq!(resolved.invocation, alias);
        assert_eq!(resolved.canonical, target);
    }
    let local = cwd.join("local");
    std::os::unix::fs::symlink(&target, &local).unwrap();
    assert_eq!(
        executable_in_path("local", &cwd, std::ffi::OsStr::new(""))
            .unwrap()
            .invocation,
        local
    );
    assert!(executable_in_path("missing", &cwd, std::ffi::OsStr::new("")).is_err());
    assert!(executable_in_path("runtime/bin", &cwd, std::ffi::OsStr::new("")).is_err());
}
