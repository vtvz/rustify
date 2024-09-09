use std::env;

fn emit_git() {
    println!(
        "cargo:rustc-env=GIT_COMMIT_TIMESTAMP={}",
        env::var("GIT_COMMIT_TIMESTAMP").unwrap_or("<unknown>".into())
    );

    println!(
        "cargo:rustc-env=GIT_SHA={}",
        env::var("GIT_SHA").unwrap_or("<unknown>".into())
    );
}

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    emit_git();
}
