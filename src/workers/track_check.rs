use crate as rustify;
use crate::app::App;

pub async fn work() {
    // profanity::check_cases();

    rustify::infrastructure::logger::init().expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify track checking..."
    );

    let app = App::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());
    tokio::spawn(rustify::metrics::collect_daemon(app));
    tokio::spawn(rustify::metrics::prometheus_collector::collect_daemon(app));

    rustify::tick::check_playing(app).await;
}
