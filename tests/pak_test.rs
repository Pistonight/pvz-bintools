use clap::Parser as _;
use cu::pre::*;
use pvz_bintools::cli::{self, Cli};
use sha2::{Digest, Sha256};

#[test]
fn test_pak_repack_1051() -> cu::Result<()> {
    test_pak_repack("main1051")
}

#[test]
fn test_pak_repack_1065() -> cu::Result<()> {
    test_pak_repack("main1065")
}

#[test]
fn test_pak_repack_1073() -> cu::Result<()> {
    test_pak_repack("main1073")
}

#[test]
fn test_pak_repack_1096() -> cu::Result<()> {
    test_pak_repack("main1096")
}

fn test_pak_repack(name: &str) -> cu::Result<()> {
    let original = format!("data/{name}.pak");
    let directory = format!("data/{name}");
    let repack = format!("data/{name}_repack.pak");

    // read data/<name>.pak and compute sha256
    let original_hash = sha256_file(&original)?;

    // run pack CLI to a new file : data/<name>_repack.pak
    run_cli(&["pvz-bintools", "pakc", "--pack", &repack, &directory])?;

    // repack.pak should have the same sha256 as original
    let repack_hash = sha256_file(&repack)?;
    cu::ensure!(
        original_hash == repack_hash,
        "sha256 mismatch for '{name}': original {original_hash}, repack {repack_hash}"
    )?;

    Ok(())
}

fn run_cli(args: &[&str]) -> cu::Result<()> {
    let mut cli = Cli::try_parse_from(args)?;
    cli.preprocess();
    cli::run(cli)
}

fn sha256_file(path: &str) -> cu::Result<String> {
    let bytes = cu::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(digest_to_hex(&hasher.finalize()))
}

fn digest_to_hex(digest: &[u8]) -> String {
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
