use std::env;

use vergen::EmitBuilder;

fn emit_git() {
    if env::var("GIT_COMMIT_TIMESTAMP").is_err() || env::var("GIT_SHA").is_err() {
        EmitBuilder::builder()
            .all_git()
            .emit()
            .expect("Should do the trick");
        return;
    }

    println!(
        "cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP={}",
        env::var("GIT_COMMIT_TIMESTAMP").unwrap()
    );

    println!(
        "cargo:rustc-env=VERGEN_GIT_SHA={}",
        env::var("GIT_SHA").unwrap()
    );
}

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    emit_git();
}
