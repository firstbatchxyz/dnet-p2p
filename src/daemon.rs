use tokio_util::sync::CancellationToken;

use crate::DLLMP2P;

pub async fn run_daemon(cancellation: CancellationToken) {
    DLLMP2P::new().unwrap().run(cancellation, None).await;
}
