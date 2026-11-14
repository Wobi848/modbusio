//! The commit this program was built from, shown beside the version (*Settings → About*,
//! `--version`), so builds between two releases can be told apart. Empty where git is missing.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let commit = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_default();
    println!("cargo:rustc-env=MODBUSIO_COMMIT={commit}");
    // built again when the commit changes: a checkout moves HEAD, every commit writes its log
    for path in ["HEAD", "logs/HEAD"] {
        if let Some(file) = git(&["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={file}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
