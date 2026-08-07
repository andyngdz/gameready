use tempfile::TempDir;

use super::*;

/// Sets the fake-root variable for one test and puts it back afterwards.
///
/// The variable is process-wide, so these tests take a lock rather than run
/// beside each other: two of them setting it at once would each see the
/// other's value.
struct FakeRoot {
    _dir: TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl FakeRoot {
    fn set() -> Self {
        let guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().expect("temp dir");
        // SAFETY: the lock above makes this the only thread touching the
        // environment for the length of the test.
        unsafe { std::env::set_var(FAKE_ROOT, dir.path()) };
        Self {
            _dir: dir,
            _guard: guard,
        }
    }
}

impl Drop for FakeRoot {
    fn drop(&mut self) {
        // SAFETY: same lock, still held.
        unsafe { std::env::remove_var(FAKE_ROOT) };
    }
}

#[test]
fn the_fake_root_variable_wins_over_the_real_machine() {
    let _root = FakeRoot::set();

    let machine = Machine::detect(Effect::Reads).expect("detected");

    assert!(matches!(machine, Machine::Fixture(_)), "{machine:?}");
}

#[test]
fn a_fixture_needs_no_password_even_for_a_command_that_would_change_things() {
    // Otherwise a snapshot of a run that mutates could not be taken without a
    // password prompt, which is the thing a snapshot cannot answer.
    let _root = FakeRoot::set();

    let machine = Machine::detect(Effect::Mutates).expect("detected");

    assert!(machine.authorize().is_ok());
}
