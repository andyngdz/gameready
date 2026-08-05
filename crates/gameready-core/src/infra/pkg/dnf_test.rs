use crate::infra::exec::MockRunner;

use super::*;

const INSTALLED_QUERY: &str = "rpm -q --queryformat=%{VERSION}-%{RELEASE} gamemode";

#[test]
fn reports_an_installed_package_with_its_version() {
    let runner = MockRunner::new().answering(INSTALLED_QUERY, "1.8.2-3.fc41");
    let state = Dnf.state(&runner, "gamemode").expect("queries");

    assert_eq!(
        state,
        PackageState::Installed {
            version: Some("1.8.2-3.fc41".to_owned())
        }
    );
}

#[test]
fn reads_installed_state_from_rpm_not_dnf() {
    // rpm reads the local database, so repository state cannot slow it down or
    // change the answer.
    let runner = MockRunner::new().answering(INSTALLED_QUERY, "1.8.2-3.fc41");
    let _ = Dnf.state(&runner, "gamemode").expect("queries");

    assert_eq!(runner.commands(), [INSTALLED_QUERY]);
}

#[test]
fn reports_a_package_missing_from_every_repository() {
    let runner = MockRunner::new().failing("dnf info --quiet not-a-package");
    assert_eq!(
        Dnf.state(&runner, "not-a-package").expect("queries"),
        PackageState::Unavailable
    );
}

#[test]
fn the_install_command_is_non_interactive() {
    let runner = MockRunner::new().answering("dnf info --quiet mangohud", "Name : mangohud\n");
    let _ = Dnf
        .install(&runner, &["mangohud".to_owned()])
        .expect("installs");

    assert!(
        runner
            .commands()
            .contains(&"sudo dnf install --assumeyes mangohud".to_owned()),
        "unexpected commands: {:?}",
        runner.commands()
    );
}
