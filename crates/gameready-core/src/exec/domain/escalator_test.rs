use super::*;

#[test]
fn prefers_sudo_when_several_are_present() {
    let found = Escalator::detect(|binary| matches!(binary, "sudo" | "doas" | "run0"))
        .expect("one is present");
    assert_eq!(found, Escalator::Sudo);
}

#[test]
fn falls_back_to_doas_when_sudo_is_absent() {
    let found = Escalator::detect(|binary| binary == "doas").expect("doas is present");
    assert_eq!(found, Escalator::Doas);
}

#[test]
fn reports_what_it_looked_for_when_nothing_is_available() {
    let error = Escalator::detect(|_| false).expect_err("nothing present");
    let message = error.to_string();
    assert!(
        message.contains("sudo"),
        "names the tools it tried: {message}"
    );
    assert!(
        message.contains("pkexec"),
        "names the tools it tried: {message}"
    );
}

#[test]
fn only_sudo_claims_to_cache_credentials() {
    assert!(Escalator::Sudo.caches_credentials());
    // The "asked once" promise is only true for sudo, so the others must not
    // claim it and make the pre-flight screen lie.
    assert!(!Escalator::Doas.caches_credentials());
    assert!(!Escalator::Run0.caches_credentials());
    assert!(!Escalator::Pkexec.caches_credentials());
}

#[test]
fn wraps_a_command_with_non_interactive_sudo() {
    let (program, args) = Escalator::Sudo.wrap("sysctl", &["-w".to_owned(), "a=b".to_owned()]);
    assert_eq!(program, "sudo");
    // -n so a stale credential cache surfaces as an error rather than as a
    // password prompt under a progress display.
    assert_eq!(args, ["-n", "sysctl", "-w", "a=b"]);
}

#[test]
fn doas_takes_no_non_interactive_flag() {
    let (program, args) = Escalator::Doas.wrap("rm", &["-f".to_owned()]);
    assert_eq!(program, "doas");
    assert_eq!(args, ["rm", "-f"]);
}
