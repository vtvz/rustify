use rustify::state::AppState;

async fn run() {
    rustify::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify metrics collection..."
    );

    let app = AppState::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());

    rustify::metrics::collect_daemon(app).await;
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
