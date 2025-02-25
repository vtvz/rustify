use anyhow::Context as _;
use rustify::app::App;
use rustify::queue::profanity_check;
use rustify::utils;
use tokio::task::JoinHandle;

async fn run() {
    rustify::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify track checking..."
    );

    let app = App::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());

    let handler: JoinHandle<anyhow::Result<_>> = tokio::spawn(async move {
        loop {
            let redis = app.redis_conn().await.context("Connection anavailable")?;

            tokio::select! {
                _ = utils::ctrl_c() => break,

                res = profanity_check::consume(app, redis) => {
                    if let Err(err) = res {
                        tracing::error!(err = ?err, "Error on profanity_check queue process");
                    }
                }
            }
        }

        Ok(())
    });

    handler.await.expect("Should work").expect("Should work");
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
