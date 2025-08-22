use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn register_evm_lists_channel_ops() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "register", "evm", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Initialize channel contract")
                .and(predicate::str::contains("Reset channel contract")),
        );
}

#[test]
fn register_sora_evm_lists_app_asset_channel() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "register", "sora", "evm", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Register EVM app")
                .and(predicate::str::contains("Register asset"))
                .and(predicate::str::contains("Register EVM channel")),
        );
}

#[test]
fn relay_evm_help_present() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "evm", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("EVM to SORA relay"));
}

#[test]
fn relay_sora_help_includes_evm_ton() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "sora", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("SORA to EVM relay")
                .and(predicate::str::contains("SORA to TON relay")),
        );
}

#[test]
fn register_ton_lists_reset() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "register", "ton", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset channel contract"));
}

#[test]
fn register_parachain_help_includes_beefy_and_trusted() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "register", "parachain", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Initialize BEEFY light client for parachain")
                .and(predicate::str::contains("Register trusted peers for parachain")),
        );
}

#[test]
fn relay_parachain_help_includes_sora_and_parachain() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "parachain", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Parachain to SORA relay commands")
                .and(predicate::str::contains("Parachain to parachain relay commands")),
        );
}

#[test]
fn relay_liberland_help_includes_both_directions() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "liberland", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Liberland to SORA relay commands")
                .and(predicate::str::contains("SORA to Liberland relay commands")),
        );
}

#[test]
fn relay_sora_sora_help_includes_trusted_and_beefy() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "sora", "sora", "--help"]) // nested
        .assert()
        .success()
        .stdout(
            predicate::str::contains("trusted").and(predicate::str::contains("BEEFY")),
        );
}

#[test]
fn relay_parachain_sora_help_includes_trusted_and_beefy() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "relay", "parachain", "sora", "--help"]) // nested
        .assert()
        .success()
        .stdout(
            predicate::str::contains("trusted").and(predicate::str::contains("BEEFY")),
        );
}

#[test]
fn old_bridge_top_level_help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["old-bridge", "--help"]) // top-level subcommand
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Register bridge")
                .and(predicate::str::contains("Register assets"))
                .and(predicate::str::contains("Migrate")),
        );
}

#[test]
fn register_liberland_help_lists_trusted() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "register", "liberland", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("trusted"));
}

#[test]
fn transfer_help_includes_evm_and_ton() {
    let mut cmd = Command::cargo_bin("bridge-relayer").expect("binary exists");
    cmd.args(["bridge", "transfer", "--help"]) // lists subcommands
        .assert()
        .success()
        .stdout(predicate::str::contains("evm").and(predicate::str::contains("ton")));
}
