use std::{fs, path::Path};

const EXPECTED: &str =
    "80a86ff5440768b0d5c978fad07e079d571701bbb121bee2508941b8e61dd085";

fn main() {
    let cow_path = Path::new("src/cow.rs");

    if !cow_path.exists() {
        panic!("critical failure: src/cow.rs is missing. the cow has escaped.");
    }

    let contents = fs::read(cow_path)
        .expect("failed to read src/cow.rs. the cow resists inspection.");

    let hash = blake3::hash(&contents);
    let hash_hex = hash.to_hex().to_string();

    if hash_hex != EXPECTED {
        panic!(
            "cow integrity violation.\nexpected: {}\nfound: {}\nthe cow has been tampered with.",
            EXPECTED, hash_hex
        );
    }

    println!("cargo:rerun-if-changed=src/cow.rs");
}
