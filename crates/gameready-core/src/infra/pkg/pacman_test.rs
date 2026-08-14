use crate::infra::exec::MockRunner;

use super::*;

#[test]
fn reports_an_installed_package_with_its_version() {
    let runner = MockRunner::new().answering("pacman -Q gamemode", "gamemode 1.8.2-1");
    let state = Pacman.state(&runner, "gamemode").expect("queries");

    assert_eq!(
        state,
        PackageState::Installed {
            version: Some("1.8.2-1".to_owned())
        }
    );
}

#[test]
fn reports_an_uninstalled_but_available_package() {
    let runner = MockRunner::new()
        .failing("pacman -Q lutris")
        .answering("pacman -Si lutris", "Repository : extra\n");
    let state = Pacman.state(&runner, "lutris").expect("queries");

    assert_eq!(state, PackageState::Available);
}

#[test]
fn the_install_never_refreshes_the_database() {
    // -Sy inside an install is how a partial upgrade happens on Arch, and a
    // partial upgrade is the classic way to break the system.
    let runner = MockRunner::new()
        .failing("pacman -Q mangohud")
        .answering("pacman -Si mangohud", "Repository : extra\n");
    let _ = Pacman
        .install(&runner, &["mangohud".to_owned()])
        .expect("installs");

    let install = runner
        .commands()
        .into_iter()
        .find(|cmd| cmd.contains("-S "))
        .expect("an install ran");

    assert_eq!(install, "sudo pacman -S --needed --noconfirm mangohud");
    assert!(!install.contains("-Sy"), "refreshed mid-install: {install}");
}

#[test]
fn reports_a_package_missing_from_every_repository() {
    let runner = MockRunner::new()
        .failing("pacman -Q not-a-package")
        .failing("pacman -Si not-a-package");

    assert_eq!(
        Pacman.state(&runner, "not-a-package").expect("queries"),
        PackageState::Unavailable
    );
}
