use crate::Error;
use baylee_tower::v2::{
    execution::MAX_CHUNK_BYTES,
    verification::{MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, Response},
};
use baylee_tower_store::{Principal, Store, VerificationLease};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _};

pub(crate) async fn write(
    mut pipe: tokio::process::ChildStdin,
    header: Vec<u8>,
    store: &Store,
    principal: &Principal,
    lease: VerificationLease,
) -> Result<(), Error> {
    frame(&mut pipe, &header, MAX_REQUEST_BYTES).await?;
    for index in 0..lease.binding.prefix.chunks {
        let chunk = store.verification_chunk(principal, lease, index).await?;
        frame(&mut pipe, &chunk.data, MAX_CHUNK_BYTES).await?;
    }
    pipe.shutdown().await.map_err(|_| Error::Process)?;
    drop(pipe);
    Ok(())
}

async fn frame(
    pipe: &mut tokio::process::ChildStdin,
    bytes: &[u8],
    limit: usize,
) -> Result<(), Error> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| Error::Contract)?
        .to_be_bytes();
    baylee_worker::frame::length(length, limit).map_err(|_| Error::Contract)?;
    pipe.write_all(&length).await.map_err(|_| Error::Process)?;
    pipe.write_all(bytes).await.map_err(|_| Error::Process)?;
    Ok(())
}

pub(crate) async fn read(mut pipe: impl AsyncRead + Unpin) -> Result<Response, Error> {
    let mut length = [0; 4];
    pipe.read_exact(&mut length)
        .await
        .map_err(|_| Error::Contract)?;
    let length =
        baylee_worker::frame::length(length, MAX_RESPONSE_BYTES).map_err(|_| Error::Contract)?;
    let mut bytes = vec![0; length];
    pipe.read_exact(&mut bytes)
        .await
        .map_err(|_| Error::Contract)?;
    let response = serde_json::from_slice(&bytes).map_err(|_| Error::Contract)?;
    if pipe.read(&mut [0]).await.map_err(|_| Error::Process)? != 0 {
        return Err(Error::Contract);
    }
    Ok(response)
}
