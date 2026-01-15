use std::time::Duration;

use apalis::layers::WorkerBuilderExt as _;
use apalis::layers::retry::RetryPolicy;
use apalis::prelude::{Monitor, WorkerBuilder};

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

    Monitor::new()
        .register(move |_| {
            WorkerBuilder::new("rustify:profanity_check")
                .backend(app.queue_manager().profanity_queue())
                .concurrency(2)
                // Ordering of timeout and retry matters!
                .timeout(Duration::from_secs(90))
                .retry(RetryPolicy::retries(2))
                .data(app)
                .build(profanity_check::consume)
        })
        .run()
        .await
        .expect("Should Work");
}
