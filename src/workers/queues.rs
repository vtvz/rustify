use tokio::task::JoinHandle;

use crate as rustify;
use crate::app::App;
use crate::queue::profanity_check;

pub async fn work() {
    rustify::infrastructure::logger::init().expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify track checking..."
    );

    let app = App::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());

    let handler: JoinHandle<anyhow::Result<_>> = tokio::spawn(profanity_check::worker(app));

    handler.await.expect("Should work").expect("Should work");
}
