use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn help_includes_sora_ton_subcommand() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "sora", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TON").and(predicate::str::contains("SORA to TON relay")));
}

#[test]
fn sora_ton_help_shows_flags() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "sora", "ton", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--value")
                .and(predicate::str::contains("--no-bounce")),
        );
}

#[test]
fn sora_evm_help_present() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "sora", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("SORA to EVM relay"));
}

#[test]
fn ton_sora_help_present() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "ton", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("TON to SORA relay"));
}
