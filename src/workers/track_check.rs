use crate::app::App;

pub async fn work() {
    // profanity::check_cases();

    crate::logger::init().await.expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify track checking..."
    );

    let app = App::init().await.expect("State to be built");

    tokio::spawn(crate::utils::listen_for_ctrl_c());
    tokio::spawn(crate::metrics::collect_daemon(app));

    crate::tick::check_playing(app).await;
}
