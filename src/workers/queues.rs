use anyhow::Context as _;
use tokio::task::JoinHandle;

use crate as rustify;
use crate::app::App;
use crate::queue::profanity_check;
use crate::utils;

pub async fn work() {
    rustify::infrastructure::logger::init().expect("Logger should be built");

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
                () = utils::ctrl_c() => break,

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
