//! A fixed list of independent duels across threads. Each job carries its own
//! seed, so the results are the same whatever the scheduling and thread count.
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// Run `work` over `jobs` on `threads` threads; results stay in job order.
/// `progress` sees each finished count, for an operator's log line.
pub fn run<J: Sync, R: Send>(
    jobs: &[J],
    threads: usize,
    work: impl Fn(&J) -> R + Sync,
    progress: impl Fn(usize, usize) + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..jobs.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(index) else { break };
                    let result = work(job);
                    results.lock().expect("no worker panics while holding results")[index] =
                        Some(result);
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1, jobs.len());
                }
            });
        }
    });
    results
        .into_inner()
        .expect("every worker has finished")
        .into_iter()
        .map(|result| result.expect("every job ran"))
        .collect()
}
