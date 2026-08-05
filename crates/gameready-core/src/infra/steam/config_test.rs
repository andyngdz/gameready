use std::time::Duration;

use indoc::indoc;
use tempfile::TempDir;

use super::*;

fn account(root: &TempDir, id: &str) -> PathBuf {
    let dir = root.path().join(USERDATA).join(id).join("config");
    std::fs::create_dir_all(&dir).expect("create account dir");
    let path = dir.join("localconfig.vdf");
    std::fs::write(
        &path,
        indoc! {r#"
            "UserLocalConfigStore"
            {
            }
        "#},
    )
    .expect("write config");
    path
}

#[test]
fn the_only_account_is_the_one_chosen() {
    let root = TempDir::new().expect("temp dir");
    let expected = account(&root, "172354714");

    assert_eq!(local_config_under(root.path()).expect("found"), expected);
}

#[test]
fn the_most_recently_used_account_wins() {
    // Nothing in the file tree says which account is logged in. The one written
    // last is the one that used this machine most recently.
    let root = TempDir::new().expect("temp dir");
    account(&root, "111");
    std::thread::sleep(Duration::from_millis(20));
    let newer = account(&root, "222");

    assert_eq!(local_config_under(root.path()).expect("found"), newer);
}

#[test]
fn a_steam_with_no_accounts_is_reported_rather_than_guessed_at() {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join(USERDATA)).expect("create userdata");

    let error = local_config_under(root.path()).expect_err("no account");
    assert!(matches!(error, SteamError::NoUserConfig), "{error:?}");
}

#[test]
fn a_directory_that_is_not_steam_is_reported() {
    let root = TempDir::new().expect("temp dir");
    let error = local_config_under(root.path()).expect_err("not steam");
    assert!(matches!(error, SteamError::NoUserConfig), "{error:?}");
}

#[test]
fn an_account_directory_without_a_config_is_skipped() {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join(USERDATA).join("333")).expect("create dir");
    let real = account(&root, "444");

    assert_eq!(local_config_under(root.path()).expect("found"), real);
}
