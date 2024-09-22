use rustify::state::AppState;

async fn run() {
    // profanity::check_cases();

    rustify::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify track checking..."
    );

    let app_state = AppState::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());
    rustify::tick::check_playing(app_state).await;
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
