use vergen::EmitBuilder;

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    EmitBuilder::builder()
        .all_build()
        .all_git()
        .emit()
        .expect("Should do the trick");
}
