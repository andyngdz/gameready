use crate::infra::exec::MockRunner;

use super::*;

const INSTALLED_QUERY: &str = "dpkg-query --showformat=${Version} --show gamemode";

#[test]
fn reports_an_installed_package_with_its_version() {
    let runner = MockRunner::new().answering(INSTALLED_QUERY, "1.8.2-2build1");
    let state = Apt.state(&runner, "gamemode").expect("queries");

    assert_eq!(
        state,
        PackageState::Installed {
            version: Some("1.8.2-2build1".to_owned())
        }
    );
}

#[test]
fn reads_installed_state_from_dpkg_not_apt() {
    // A package stays installed after its repository disappears. Asking apt
    // would call that unavailable and offer to install what is already there.
    let runner = MockRunner::new().answering(INSTALLED_QUERY, "1.8.2-2build1");
    let _ = Apt.state(&runner, "gamemode").expect("queries");

    assert_eq!(runner.commands(), [INSTALLED_QUERY]);
}

#[test]
fn reports_an_uninstalled_but_available_package() {
    let runner = MockRunner::new().answering("apt-cache show mangohud", "Package: mangohud\n");
    let state = Apt.state(&runner, "mangohud").expect("queries");

    assert_eq!(state, PackageState::Available);
}

#[test]
fn reports_a_package_missing_from_every_repository() {
    // A package absent from every configured repository is not failing: a step
    // needing it becomes NotApplicable rather than failing there.
    let runner = MockRunner::new().failing("apt-cache show missing-package");
    let state = Apt.state(&runner, "missing-package").expect("queries");

    assert_eq!(state, PackageState::Unavailable);
}

#[test]
fn installs_only_what_is_not_already_present() {
    let runner = MockRunner::new()
        .answering(INSTALLED_QUERY, "1.8.2-2build1")
        .answering("apt-cache show mangohud", "Package: mangohud\n");

    let outcome = Apt
        .install(&runner, &["gamemode".to_owned(), "mangohud".to_owned()])
        .expect("installs");

    // gamemode was already there, so removing it later is not gameready's to do.
    assert_eq!(outcome.newly_installed, ["mangohud"]);
    assert_eq!(outcome.requested.len(), 2);
}

#[test]
fn the_install_command_is_non_interactive_and_skips_recommends() {
    let runner = MockRunner::new().answering("apt-cache show mangohud", "Package: mangohud\n");
    let _ = Apt
        .install(&runner, &["mangohud".to_owned()])
        .expect("installs");

    assert!(
        runner
            .commands()
            .contains(&"sudo apt-get install --yes --no-install-recommends mangohud".to_owned()),
        "unexpected commands: {:?}",
        runner.commands()
    );
}

#[test]
fn installing_nothing_runs_no_privileged_command() {
    let runner = MockRunner::new().answering(INSTALLED_QUERY, "1.8.2-2build1");
    let outcome = Apt
        .install(&runner, &["gamemode".to_owned()])
        .expect("installs");

    assert!(!outcome.changed_anything());
    assert!(
        !runner.commands().iter().any(|cmd| cmd.starts_with("sudo")),
        "asked for root with nothing to do: {:?}",
        runner.commands()
    );
}
