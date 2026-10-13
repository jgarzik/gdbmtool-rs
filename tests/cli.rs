// End-to-end tests of the command line tools.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const GDBMTOOL: &str = env!("CARGO_BIN_EXE_gdbmtool-rs");
const GDBMDUMP: &str = env!("CARGO_BIN_EXE_gdbmdump-rs");

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin).args(args).output().expect("run binary")
}

// run gdbmtool-rs with commands piped to its stdin
fn run_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(GDBMTOOL)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gdbmtool-rs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

// run gdbmtool-rs on database db, expecting success, and return its output lines
fn tool(db: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec![db.to_str().unwrap()];
    all.extend_from_slice(args);
    let output = run(GDBMTOOL, &all);
    assert!(output.status.success(), "{args:?}: {}", stderr(&output));
    stdout(&output).lines().map(String::from).collect()
}

fn sorted(mut lines: Vec<String>) -> Vec<String> {
    lines.sort();
    lines
}

// a new database holding alpha => one, beta => two
fn new_db(dir: &Path) -> std::path::PathBuf {
    let db = dir.join("test.db");
    let output = run(
        GDBMTOOL,
        &["-c", db.to_str().unwrap(), "insert", "alpha", "one"],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    tool(&db, &["insert", "beta", "two"]);
    db
}

#[test]
fn insert_get_remove() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    assert_eq!(tool(&db, &["get", "alpha"]), ["one"]);
    assert_eq!(tool(&db, &["get", "missing"]), Vec::<String>::new());
    assert_eq!(tool(&db, &["len"]), ["2"]);

    // insert replaces, showing the old value
    assert_eq!(tool(&db, &["insert", "alpha", "uno"]), ["one"]);
    assert_eq!(tool(&db, &["get", "alpha"]), ["uno"]);

    assert_eq!(tool(&db, &["remove", "alpha"]), ["uno"]);
    assert_eq!(tool(&db, &["remove", "alpha"]), Vec::<String>::new());
    assert_eq!(tool(&db, &["len"]), ["1"]);
}

#[test]
fn try_insert_keeps_existing() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    assert_eq!(tool(&db, &["try-insert", "beta", "zwei"]), ["two"]);
    assert_eq!(tool(&db, &["get", "beta"]), ["two"]);
    assert_eq!(
        tool(&db, &["try-insert", "gamma", "three"]),
        Vec::<String>::new()
    );
    assert_eq!(tool(&db, &["get", "gamma"]), ["three"]);
}

#[test]
fn listing() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    assert_eq!(sorted(tool(&db, &["keys"])), ["alpha", "beta"]);
    assert_eq!(sorted(tool(&db, &["values"])), ["one", "two"]);
    assert_eq!(
        sorted(tool(&db, &["entries"])),
        ["alpha => one", "beta => two"]
    );
}

#[test]
fn read_only_rejects_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let output = run(GDBMTOOL, &["-r", db.to_str().unwrap(), "insert", "x", "y"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("read-only"), "{}", stderr(&output));
    assert_eq!(tool(&db, &["len"]), ["2"]);
}

#[test]
fn missing_database_fails() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("missing.db");

    let output = run(GDBMTOOL, &[db.to_str().unwrap(), "len"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Failed to open database"));
    assert!(!db.exists());
}

#[test]
fn command_stream() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let output = run_stdin(
        &[db.to_str().unwrap()],
        "insert gamma \"three and more\"\nget gamma\nlen\n",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "three and more\n3\n");

    // the stream stops at the first failing command
    let output = run_stdin(&[db.to_str().unwrap()], "len\nbogus\nlen\n");
    assert!(!output.status.success());
    assert_eq!(stdout(&output), "3\n");
}

#[test]
fn dump_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());
    let dump = dir.path().join("test.dump");

    tool(&db, &["dump", dump.to_str().unwrap()]);
    let text = std::fs::read_to_string(&dump).unwrap();
    assert!(text.starts_with("# GDBM dump file created by gdbm-native"));

    let copy = dir.path().join("copy.db");
    let output = run(
        GDBMTOOL,
        &["-c", copy.to_str().unwrap(), "load", dump.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        sorted(tool(&copy, &["entries"])),
        ["alpha => one", "beta => two"]
    );
}

#[test]
fn header_and_directory() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let header = tool(&db, &["header"]);
    assert!(header[0].starts_with("magic "), "{header:?}");
    assert!(!tool(&db, &["dir"]).is_empty());
}

#[test]
fn block_size() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");

    let output = run(
        GDBMTOOL,
        &["-c", "-b", "1000", db.to_str().unwrap(), "insert", "k", "v"],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    // rounded up to a power of two
    assert!(tool(&db, &["header"]).contains(&"block-size 1024".to_string()));
}

#[test]
fn gdbmdump_ascii_and_binary() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());
    let ascii = dir.path().join("ascii.dump");
    let binary = dir.path().join("binary.dump");

    let output = run(
        GDBMDUMP,
        &[
            "--format",
            "ascii",
            db.to_str().unwrap(),
            ascii.to_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        std::fs::read_to_string(&ascii)
            .unwrap()
            .starts_with("# GDBM dump file")
    );

    let output = run(GDBMDUMP, &[db.to_str().unwrap(), binary.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        std::fs::read(&binary)
            .unwrap()
            .starts_with(b"!\r\n! GDBM FLAT FILE DUMP")
    );
}
