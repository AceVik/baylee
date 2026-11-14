use super::*;
use baylee_worker::{ALIVE, BUSY, IDLE};
use tokio::io::AsyncWriteExt as _;

#[tokio::test]
async fn private_progress_requires_exact_completion_and_bounds_chatter() {
    let limits = baylee_worker::Limits::default();
    assert_eq!(
        progress(&[IDLE, BUSY, ALIVE, IDLE, ALIVE][..], limits).await,
        Ok(())
    );
    for bytes in [
        &[][..],
        &[IDLE],
        &[IDLE, BUSY],
        &[BUSY, IDLE],
        &[ALIVE],
        &[IDLE, IDLE],
        &[IDLE, BUSY, BUSY, IDLE],
        &[IDLE, BUSY, IDLE, BUSY],
    ] {
        assert_eq!(progress(bytes, limits).await, Err(Error::Contract));
    }
    let mut noisy = vec![IDLE, BUSY];
    noisy.extend(std::iter::repeat_n(ALIVE, 512));
    noisy.push(IDLE);
    assert_eq!(
        progress(noisy.as_slice(), limits).await,
        Err(Error::Contract)
    );
}

#[tokio::test]
async fn a_stalled_operation_is_bounded_even_while_the_pipe_remains_open() {
    let (mut input, output) = tokio::io::duplex(16);
    input.write_all(&[IDLE, BUSY, ALIVE]).await.unwrap();
    let limits = baylee_worker::Limits {
        operation: Duration::ZERO,
        ..Default::default()
    };
    let observed = tokio::time::timeout(Duration::from_secs(1), progress(output, limits))
        .await
        .unwrap();
    assert_eq!(observed, Err(Error::Budget));
    drop(input);
}

#[tokio::test]
async fn results_reject_oversized_truncated_unknown_and_trailing_payloads() {
    assert_eq!(read(&[255; 4][..]).await, Err(Error::Contract));
    assert_eq!(read(&[0, 0, 0, 8, 1][..]).await, Err(Error::Contract));
    let mut bytes = Vec::new();
    baylee_worker::frame::write(
        &mut bytes,
        br#"{"kind":"rejected","reason":"invalid"}"#,
        8192,
    )
    .unwrap();
    assert!(read(bytes.as_slice()).await.is_ok());
    bytes.push(0);
    assert_eq!(read(bytes.as_slice()).await, Err(Error::Contract));
    bytes.clear();
    baylee_worker::frame::write(
        &mut bytes,
        br#"{"kind":"rejected","reason":"invalid","secret":"never echoed"}"#,
        8192,
    )
    .unwrap();
    assert_eq!(read(bytes.as_slice()).await, Err(Error::Contract));
}
