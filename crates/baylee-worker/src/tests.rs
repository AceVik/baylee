use super::*;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn budget() -> Budget {
    Budget::new(
        Limits {
            startup: ms(10),
            operation: ms(20),
            resident_bytes: 100,
        },
        ms(0),
    )
}

#[test]
fn idle_thinking_has_no_deadline_but_stalled_work_and_memory_do() {
    let mut budget = budget();
    assert_eq!(budget.check(ms(9), 99), Ok(()));
    assert_eq!(budget.check(ms(10), 0), Err(Failure::Startup));
    budget.observe(IDLE, ms(1)).unwrap();
    assert_eq!(budget.check(ms(1_000_000), 99), Ok(()));
    budget.observe(BUSY, ms(1_000_001)).unwrap();
    assert_eq!(budget.check(ms(1_000_020), 99), Ok(()));
    assert_eq!(budget.check(ms(1_000_021), 0), Err(Failure::Operation));
    assert_eq!(budget.observe(IDLE, ms(1_000_021)), Err(Failure::Operation));
    assert_eq!(budget.check(ms(1_000_020), 100), Err(Failure::Memory));
}

#[test]
fn malformed_and_repeated_markers_cannot_renew_work() {
    let mut budget = budget();
    assert_eq!(budget.observe(BUSY, ms(0)), Err(Failure::Progress));
    assert_eq!(budget.observe(255, ms(0)), Err(Failure::Progress));
    budget.observe(IDLE, ms(1)).unwrap();
    assert_eq!(budget.observe(IDLE, ms(2)), Err(Failure::Progress));
    budget.observe(BUSY, ms(3)).unwrap();
    assert_eq!(budget.observe(BUSY, ms(20)), Err(Failure::Progress));
    assert_eq!(budget.check(ms(23), 0), Err(Failure::Operation));
}

#[test]
fn productive_operations_are_not_mistaken_for_an_endless_loop() {
    let mut bytes = Vec::new();
    {
        let mut reporter = Reporter::new(Some(&mut bytes)).unwrap();
        for _ in 0..1_000 {
            assert_eq!(reporter.work(|| 17).unwrap(), 17);
        }
    }
    let mut budget = budget();
    for (index, marker) in bytes.into_iter().enumerate() {
        budget.observe(marker, ms(index as u64)).unwrap();
    }
    assert_eq!(budget.check(ms(10_000), 99), Ok(()));
}

#[test]
fn a_broken_owner_pipe_prevents_more_rules_work() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert!(Reporter::new(Some(Broken)).is_err());
    let mut reporter = Reporter(Some(Broken));
    assert!(reporter.work(|| panic!("must not run")).is_err());
    let mut disabled = Reporter::<Broken>::new(None).unwrap();
    assert_eq!(disabled.work(|| 23).unwrap(), 23);
}
