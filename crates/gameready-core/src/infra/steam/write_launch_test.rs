use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::Family;
use crate::games::AppId;
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths};

const CONFIG: &str = "/steam/userdata/1/config/localconfig.vdf";

fn config_text() -> String {
    indoc! {r#"
        "UserLocalConfigStore"
        {
        	"Software"
        	{
        		"Valve"
        		{
        			"Steam"
        			{
        				"apps"
        				{
        					"1422450"
        					{
        						"LastPlayed"		"1"
        					}
        				}
        			}
        		}
        	}
        }
    "#}
    .to_owned()
}

/// A machine with Steam already stopped, so no shutdown wait is involved.
fn stopped_steam() -> MockRunner {
    MockRunner::new()
        .failing("pgrep -x steam")
        .with_file(CONFIG, config_text())
}

fn target() -> LaunchTarget {
    LaunchTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        options: "gamemoderun %command%".to_owned(),
    }
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("journal")
}

#[test]
fn the_options_are_written_once_steam_is_stopped() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    let report = write_launch_options(
        &runner,
        &facts,
        &mut journal(&dir),
        CONFIG.into(),
        vec![target()],
    )
    .expect("written");

    assert_eq!(report.steps.len(), 1, "the launch-options step did not run");
    assert!(
        runner
            .file(CONFIG)
            .expect("config")
            .contains("gamemoderun %command%")
    );
}

#[test]
fn a_steam_that_is_already_stopped_is_not_asked_to_quit() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    write_launch_options(
        &runner,
        &facts,
        &mut journal(&dir),
        CONFIG.into(),
        vec![target()],
    )
    .expect("written");

    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("-shutdown")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn no_targets_changes_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    write_launch_options(
        &runner,
        &facts,
        &mut journal(&dir),
        CONFIG.into(),
        Vec::new(),
    )
    .expect("nothing to do");

    assert_eq!(runner.file(CONFIG).as_deref(), Some(config_text().as_str()));
}
