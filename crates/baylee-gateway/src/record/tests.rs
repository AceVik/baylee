use super::*;
use std::sync::Arc;

fn piece(game_id: &str, seq: u32, bytes: usize) -> v1::GameRecordChunk {
    v1::GameRecordChunk {
        game_id: game_id.to_owned(),
        seq,
        data: vec![0; bytes],
        last: false,
    }
}

fn handed(rx: &mut mpsc::UnboundedReceiver<Entry>) -> Vec<u32> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .map(|e| match e {
            Entry::Piece(p) => p.seq,
            Entry::Flushed(_) | Entry::Ended(_) => u32::MAX,
        })
        .collect()
}

/// Only this game's pieces go to the writer, and only up to
/// [`MAX_RECORD_BYTES`] of them: the piece that crosses it and every one
/// after are dropped, however small.
#[test]
fn a_sink_takes_its_own_game_up_to_the_bound() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut sink = Sink::feeding("g1", tx);
    sink.take(piece("g1", 0, 10));
    sink.take(piece("g2", 1, 10));
    sink.take(piece("", 2, 10));
    assert_eq!(handed(&mut rx), [0], "another game's piece is dropped");

    sink.take(piece("g1", 1, MAX_RECORD_BYTES - 10));
    assert_eq!(handed(&mut rx), [1], "exactly the bound fits");
    sink.take(piece("g1", 2, 1));
    sink.take(piece("g1", 3, 0));
    assert!(handed(&mut rx).is_empty(), "past the bound nothing goes");
    // A report still hears that this is all there is.
    sink.flushed(oneshot::channel().0);
    assert_eq!(handed(&mut rx), [u32::MAX]);
}

/// What a [`Store`] was asked to do, in order.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn said(&self) -> Vec<String> {
        self.0.lock().clone()
    }
}

/// A store that gives the runtime away several times over every write,
/// as a busy database does, so a flush that raced its pieces would be
/// seen resolving first.
struct Slow(Log);

async fn dawdle() {
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
}

impl Store for Slow {
    async fn open(&mut self) -> Result<(), String> {
        dawdle().await;
        self.0.0.lock().push("open".to_owned());
        Ok(())
    }

    async fn append(&mut self, piece: v1::GameRecordChunk) -> Result<(), String> {
        dawdle().await;
        self.0.0.lock().push(format!("piece {}", piece.seq));
        Ok(())
    }

    async fn finish(&mut self, ending: Ending) -> Result<(), String> {
        dawdle().await;
        self.0.0.lock().push(format!("ended {:?}", ending.winners));
        Ok(())
    }
}

/// A flush resolves only once every piece handed in before it has been
/// written (#323): that is what lets a report read the store the moment
/// it resolves and find the moment it asked for. One handed in after it
/// is not waited for.
#[tokio::test]
async fn a_flush_resolves_after_the_pieces_before_it_are_written() {
    let log = Log::default();
    let mut sink = Sink::writing("g1", Slow(log.clone()));
    sink.take(piece("g1", 0, 10));
    sink.take(piece("g1", 1, 10));
    let (done, flushed) = oneshot::channel();
    sink.flushed(done);
    sink.take(piece("g1", 2, 10));

    flushed.await.expect("the flush resolves");
    assert_eq!(log.said(), ["open", "piece 0", "piece 1"]);
    let (done, flushed) = oneshot::channel();
    sink.flushed(done);
    flushed.await.expect("the flush resolves");
    assert_eq!(log.said(), ["open", "piece 0", "piece 1", "piece 2"]);
}

/// A flush the engine answered with no piece (nothing was waiting)
/// resolves at once and starts no record: there is none to start. The
/// first piece after it does, once.
#[tokio::test]
async fn a_flush_before_any_piece_starts_no_record() {
    let log = Log::default();
    let mut sink = Sink::writing("g1", Slow(log.clone()));
    let (done, flushed) = oneshot::channel();
    sink.flushed(done);
    flushed.await.expect("the flush resolves");
    assert!(log.said().is_empty(), "{:?}", log.said());

    sink.take(piece("g1", 0, 10));
    let (done, flushed) = oneshot::channel();
    sink.flushed(done);
    sink.take(piece("g1", 1, 10));
    let (done, again) = oneshot::channel();
    sink.flushed(done);
    flushed.await.unwrap();
    again.await.unwrap();
    assert_eq!(log.said(), ["open", "piece 0", "piece 1"]);
}

/// An ask is answered only by the game it was made of, once, and a
/// forgotten one by nobody.
#[test]
fn an_ask_is_answered_by_its_own_game_once() {
    let flushes = Flushes::default();
    let (a, _rx_a) = flushes.ask("g1");
    let (b, _rx_b) = flushes.ask("g2");
    let (a, b_nonce) = (a.nonce, b.nonce);
    assert_ne!(a, b_nonce, "every ask has its own nonce");
    assert!(
        flushes.answered("g2", a).is_none(),
        "another game's engine cannot answer it"
    );
    assert!(flushes.answered("g1", a).is_some());
    assert!(flushes.answered("g1", a).is_none(), "once");
    assert_eq!(flushes.open(), 1, "the other is still open");
    drop(b);
    assert!(
        flushes.answered("g2", b_nonce).is_none(),
        "an ask whose report stopped waiting"
    );
    assert_eq!(flushes.open(), 0, "nothing left behind");
}
