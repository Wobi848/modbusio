//! The command line as a script sees it: its output and its exit code, against a simulator.

mod common;

use std::process::{Command, Output};

/// Runs modbusio with `args` and a data folder of its own, off the test's runtime.
async fn modbusio(args: Vec<String>) -> Output {
    tokio::task::spawn_blocking(move || {
        let data = std::env::temp_dir().join(format!("modbusio-cli-test-{}", std::process::id()));
        Command::new(env!("CARGO_BIN_EXE_modbusio"))
            .arg("--data-dir")
            .arg(data)
            .args(args)
            .output()
            .expect("run modbusio")
    })
    .await
    .expect("join")
}

fn words(line: &str) -> Vec<String> {
    line.split(' ').map(String::from).collect()
}

#[tokio::test]
async fn reading_nothing_ends_with_an_error() {
    let gateway = common::gateway().await;
    let read = modbusio(words(&format!("read tcp:{gateway}/2 hr 1"))).await;
    assert!(read.status.success(), "{read:?}");
    assert!(String::from_utf8_lossy(&read.stdout).contains("2500"));

    // the device behind the gateway is not there: the table says why, the exit code that it failed
    let silent = modbusio(words(&format!("read tcp:{gateway}/9 hr 0"))).await;
    assert!(!silent.status.success());
    assert!(
        String::from_utf8_lossy(&silent.stdout)
            .contains("the device behind the gateway does not answer")
    );
    assert!(String::from_utf8_lossy(&silent.stderr).contains("no value could be read"));

    let empty = modbusio(words(&format!("explore tcp:{gateway}/2 hr 50 10"))).await;
    assert!(!empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("no address answered with data"));

    let refused = modbusio(words("read tcp:127.0.0.1:1/1 hr 0")).await;
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stdout).contains("cannot connect"));
}
