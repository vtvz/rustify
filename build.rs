use vergen::{Config, vergen};

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    
    vergen(Config::default()).expect("Should do the trick");
}
