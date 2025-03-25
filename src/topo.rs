use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::DLLMP2P;

pub async fn get_topology(cancellation: CancellationToken) {
    DLLMP2P::new()
        .unwrap()
        .get_topology(cancellation, Duration::from_secs(5))
        .await;
}
