use std::process::Command;

#[test]
fn cli_reports_status_and_rejects_typesetting() {
    let binary = env!("CARGO_BIN_EXE_webloom");
    let status = Command::new(binary).arg("--version").output().unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("typesetting unsupported"));
    for args in [
        vec!["document.tex"],
        vec!["--ini"],
        vec!["--version", "document.tex"],
    ] {
        let result = Command::new(binary).args(args).output().unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8_lossy(&result.stderr).contains("unsupported request"));
    }
}
