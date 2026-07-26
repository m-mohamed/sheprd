#![cfg(unix)]

use assert_cmd::prelude::*;
use predicates::prelude::PredicateBooleanExt;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn managed_install_accepts_an_exact_version_archive_with_a_valid_checksum() {
    let fixture = InstallFixture::new();
    fixture.write_release(false);

    fixture.command().assert().success().stdout(
        predicates::str::contains("installed verified v0.2.0 binary")
            .and(predicates::str::contains("aarch64-apple-darwin")),
    );

    let installed = fixture.root.path().join("target/release/sheprd");
    assert_eq!(
        fs::read_to_string(&installed).expect("installed binary"),
        "fixture-binary\n"
    );
    assert_ne!(
        fs::metadata(installed)
            .expect("installed metadata")
            .permissions()
            .mode()
            & 0o111,
        0
    );
}

#[test]
fn managed_install_rejects_a_checksum_mismatch_without_executing_the_asset() {
    let fixture = InstallFixture::new();
    fixture.write_release(true);

    fixture
        .command()
        .assert()
        .failure()
        .stderr(predicates::str::contains("checksum mismatch"));

    assert!(!fixture.root.path().join("target/release/sheprd").exists());
}

struct InstallFixture {
    root: assert_fs::TempDir,
    releases: assert_fs::TempDir,
}

impl InstallFixture {
    fn new() -> Self {
        let root = assert_fs::TempDir::new().expect("plugin root");
        let releases = assert_fs::TempDir::new().expect("release root");
        fs::create_dir_all(root.path().join("scripts")).expect("scripts");
        fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/install-plugin.sh"),
            root.path().join("scripts/install-plugin.sh"),
        )
        .expect("installer");
        fs::write(
            root.path().join("herdr-plugin.toml"),
            "id = \"test.sheprd\"\nname = \"Sheprd\"\nversion = \"0.2.0\"\nmin_herdr_version = \"0.7.5\"\n",
        )
        .expect("manifest");
        Self { root, releases }
    }

    fn write_release(&self, bad_checksum: bool) {
        let version = self.releases.path().join("v0.2.0");
        let staging = self.releases.path().join("staging");
        fs::create_dir_all(&version).expect("version dir");
        fs::create_dir_all(&staging).expect("staging dir");
        fs::write(staging.join("sheprd"), "fixture-binary\n").expect("fixture binary");
        let archive = version.join("sheprd-aarch64-apple-darwin.tar.gz");
        let status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&staging)
            .arg("sheprd")
            .status()
            .expect("tar");
        assert!(status.success());
        let actual = format!("{:x}", Sha256::digest(fs::read(&archive).expect("archive")));
        let digest = if bad_checksum { "0".repeat(64) } else { actual };
        fs::write(
            version.join("sheprd-aarch64-apple-darwin.sha256"),
            format!("{digest}  sheprd-aarch64-apple-darwin.tar.gz\n"),
        )
        .expect("checksum");
    }

    fn command(&self) -> Command {
        let mut command = Command::new("bash");
        command
            .arg(self.root.path().join("scripts/install-plugin.sh"))
            .env("HOME", self.root.path())
            .env("SHEPRD_PLUGIN_ROOT", self.root.path())
            .env(
                "SHEPRD_RELEASE_BASE_URL",
                format!("file://{}", self.releases.path().display()),
            )
            .env("SHEPRD_UNAME_S", "Darwin")
            .env("SHEPRD_UNAME_M", "arm64");
        command
    }
}
