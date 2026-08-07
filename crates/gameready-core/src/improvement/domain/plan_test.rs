use super::*;

fn package(name: &str) -> PlannedPackage {
    PlannedPackage {
        name: name.to_owned(),
        what: "a thing".to_owned(),
        why: "a reason".to_owned(),
        approx_bytes: 1,
    }
}

#[test]
fn a_sysctl_names_both_ends_of_the_change() {
    let action = PlannedAction::SetSysctl {
        key: "vm.swappiness".to_owned(),
        from: "60".to_owned(),
        to: "180".to_owned(),
    };

    assert_eq!(action.describe(), "set vm.swappiness from 60 to 180");
}

#[test]
fn an_install_says_what_is_already_here_so_the_count_adds_up() {
    // A step called "install gamemode and mangohud" that only fetches one of
    // them otherwise looks like it half failed.
    let action = PlannedAction::InstallPackages {
        packages: vec![package("mangohud")],
        already_present: vec!["gamemode".to_owned()],
    };

    assert_eq!(
        action.describe(),
        "install mangohud (already here: gamemode)"
    );
}

#[test]
fn an_install_with_nothing_already_here_does_not_trail_an_empty_bracket() {
    let action = PlannedAction::InstallPackages {
        packages: vec![package("mangohud")],
        already_present: Vec::new(),
    };

    assert_eq!(action.describe(), "install mangohud");
}

#[test]
fn a_file_is_named_without_its_contents() {
    // The contents can be a whole drop-in, and a plan line is one line.
    let action = PlannedAction::CreateFile {
        path: "/etc/sysctl.d/99-gameready.conf".to_owned(),
        contents: "vm.swappiness = 180".to_owned(),
    };

    assert_eq!(action.describe(), "create /etc/sysctl.d/99-gameready.conf");
}
