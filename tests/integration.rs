use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_init_and_get_set() {
    let tmp = tempdir().unwrap();
    let tmp_path = tmp.path().to_str().unwrap();
    let bin = env!("CARGO_BIN_EXE_conchy-config");

    // 1. Initialize configs in temp directory
    let init_res = Command::new(bin)
        .args(["-C", tmp_path, "init"])
        .output()
        .expect("Failed to run conchy-config init");
    assert!(init_res.status.success());

    // 2. Read default layout.gaps
    let get_res = Command::new(bin)
        .args(["-C", tmp_path, "get", "wm", "layout.gaps"])
        .output()
        .expect("Failed to run conchy-config get");
    assert!(get_res.status.success());
    let gaps = String::from_utf8_lossy(&get_res.stdout).trim().to_string();
    assert_eq!(gaps, "16");

    // 3. Set layout.gaps to 32
    let set_res = Command::new(bin)
        .args(["-C", tmp_path, "set", "wm", "layout.gaps", "32"])
        .output()
        .expect("Failed to run conchy-config set");
    assert!(set_res.status.success());

    // 4. Verify value is updated
    let get_updated = Command::new(bin)
        .args(["-C", tmp_path, "get", "wm", "layout.gaps"])
        .output()
        .expect("Failed to run conchy-config get updated");
    assert!(get_updated.status.success());
    let updated_gaps = String::from_utf8_lossy(&get_updated.stdout).trim().to_string();
    assert_eq!(updated_gaps, "32");

    // 5. Check list output
    let list_res = Command::new(bin)
        .args(["-C", tmp_path, "list", "wm"])
        .output()
        .expect("Failed to run conchy-config list");
    assert!(list_res.status.success());
    let list_stdout = String::from_utf8_lossy(&list_res.stdout);
    assert!(list_stdout.contains("layout.gaps = 32"));

    // 6. Verify comments are preserved in wm.kdl
    let wm_content = fs::read_to_string(tmp.path().join("wm.kdl")).unwrap();
    assert!(wm_content.contains("// Layout and gaps"));
    assert!(wm_content.contains("gaps 32"));
}

#[test]
fn test_cli_session_config() {
    let tmp = tempdir().unwrap();
    let tmp_path = tmp.path().to_str().unwrap();
    let bin = env!("CARGO_BIN_EXE_conchy-config");

    // Init
    let _ = Command::new(bin)
        .args(["-C", tmp_path, "init"])
        .output()
        .unwrap();

    // Check session timeout
    let get_res = Command::new(bin)
        .args(["-C", tmp_path, "get", "session", "session.socket-wait-timeout"])
        .output()
        .unwrap();
    assert!(get_res.status.success());
    assert_eq!(String::from_utf8_lossy(&get_res.stdout).trim(), "10");

    // Mutate timeout
    let set_res = Command::new(bin)
        .args(["-C", tmp_path, "set", "session", "session.socket-wait-timeout", "15"])
        .output()
        .unwrap();
    assert!(set_res.status.success());

    let get_res2 = Command::new(bin)
        .args(["-C", tmp_path, "get", "session", "session.socket-wait-timeout"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&get_res2.stdout).trim(), "15");
}
