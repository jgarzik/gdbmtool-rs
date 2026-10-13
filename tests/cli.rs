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

#[test]
fn binary_data_is_escaped() {
    let dir = tempfile::tempdir().unwrap();
    let dump = dir.path().join("binary.dump");
    // key "k\xff", value "line1\nline2\\"
    std::fs::write(
        &dump,
        "# GDBM dump file\n#:version=1.1\n# End of header\n\
         #:len=2\na/8=\n#:len=12\nbGluZTEKbGluZTJc\n#:count=1\n# End of data\n",
    )
    .unwrap();

    let db = dir.path().join("test.db");
    let output = run(
        GDBMTOOL,
        &["-c", db.to_str().unwrap(), "load", dump.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(tool(&db, &["keys"]), ["k\\xff"]);
    assert_eq!(tool(&db, &["values"]), ["line1\\nline2\\\\"]);
    assert_eq!(tool(&db, &["entries"]), ["k\\xff => line1\\nline2\\\\"]);
}

#[test]
fn gdbmdump_binary_formats() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    // records are key and value lengths then data, so the dump size grows with the length field
    let size = |format: &str| {
        let output = run(
            GDBMDUMP,
            &["--bin-format", format, db.to_str().unwrap(), "-"],
        );
        assert!(output.status.success(), "{}", stderr(&output));
        output.stdout.len()
    };
    let long32 = size("long32");
    assert_eq!(size("long64le"), long32 + 4 * 4);
    assert_eq!(size("long64be"), long32 + 4 * 4);
}

#[test]
fn gdbmdump_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let output = run(GDBMDUMP, &["-f", "ascii", db.to_str().unwrap(), "-"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.starts_with("# GDBM dump file"));
    assert!(text.contains(&format!("#:file={}\n", db.display())));
    assert!(text.ends_with("#:count=2\n# End of data\n"));
}

#[test]
fn gdbmdump_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.db");
    let out = dir.path().join("out.dump");

    let output = run(
        GDBMDUMP,
        &[missing.to_str().unwrap(), out.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with("gdbmdump-rs: "),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("panicked"));
    assert!(!out.exists());

    let db = new_db(dir.path());
    let bad_out = dir.path().join("no/such/dir/out.dump");
    let output = run(GDBMDUMP, &[db.to_str().unwrap(), bad_out.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!stderr(&output).contains("panicked"));
}

// value of the "name value" line for name in lines
fn field<'a>(lines: &'a [String], name: &str) -> &'a str {
    lines
        .iter()
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(' '))
        .unwrap_or_else(|| panic!("no {name} in {lines:?}"))
}

#[test]
fn info() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let output = run(GDBMTOOL, &["-r", db.to_str().unwrap(), "info"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let lines: Vec<String> = stdout(&output).lines().map(String::from).collect();
    assert_eq!(field(&lines, "file"), db.to_str().unwrap());
    assert_eq!(field(&lines, "writable"), "no");
    assert_eq!(field(&lines, "magic"), "GDBM_NUMSYNC_MAGIC64");
    assert_eq!(field(&lines, "layout"), "Off64Align8");
    assert_eq!(field(&lines, "endian"), "Little");
    assert_eq!(field(&lines, "bucket-count"), "1");
    assert_eq!(field(&tool(&db, &["info"]), "writable"), "yes");
}

#[test]
fn verify() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let lines = tool(&db, &["verify"]);
    assert_eq!(field(&lines, "buckets"), "1");
    assert_eq!(field(&lines, "records"), "2");
}

#[test]
fn export_and_import() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    for format in ["host", "long32", "long64le", "long64be"] {
        let dump = dir.path().join(format!("{format}.dump"));
        tool(&db, &["export", "--format", format, dump.to_str().unwrap()]);

        // the format is detected on import
        let copy = dir.path().join(format!("{format}.db"));
        let output = run(
            GDBMTOOL,
            &[
                "-c",
                copy.to_str().unwrap(),
                "import",
                dump.to_str().unwrap(),
            ],
        );
        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), "2 records imported\n");
        assert_eq!(
            sorted(tool(&copy, &["entries"])),
            ["alpha => one", "beta => two"]
        );
        tool(
            &copy,
            &["import", "--format", format, dump.to_str().unwrap()],
        );
    }
}

#[test]
fn import_modes() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());
    let ascii = dir.path().join("ascii.dump");
    let binary = dir.path().join("binary.dump");
    tool(&db, &["dump", ascii.to_str().unwrap()]);
    tool(&db, &["export", binary.to_str().unwrap()]);
    tool(&db, &["insert", "alpha", "changed"]);

    // with --insert, existing keys are an error
    for (command, dump) in [("load", &ascii), ("import", &binary)] {
        let output = run(
            GDBMTOOL,
            &[
                db.to_str().unwrap(),
                command,
                "--insert",
                dump.to_str().unwrap(),
            ],
        );
        assert!(!output.status.success(), "{command}");
        assert!(
            stderr(&output).contains("already exists"),
            "{}",
            stderr(&output)
        );
    }
    assert_eq!(tool(&db, &["get", "alpha"]), ["changed"]);

    // by default they are replaced
    assert_eq!(
        tool(&db, &["load", ascii.to_str().unwrap()]),
        ["2 records loaded"]
    );
    assert_eq!(tool(&db, &["get", "alpha"]), ["one"]);
}

#[test]
fn numsync_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    tool(&db, &["downgrade"]);
    let info = tool(&db, &["info"]);
    assert_eq!(field(&info, "magic"), "GDBM_MAGIC64");
    assert_eq!(field(&info, "numsync"), "none");

    tool(&db, &["upgrade"]);
    let info = tool(&db, &["info"]);
    assert_eq!(field(&info, "magic"), "GDBM_NUMSYNC_MAGIC64");
    assert_ne!(field(&info, "numsync"), "none");
    assert_eq!(tool(&db, &["len"]), ["2"]);
}

#[test]
fn reorganize_and_sync() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let output = run_stdin(
        &[db.to_str().unwrap()],
        "insert gamma three\nremove beta\nreorganize\nsync\nlen\n",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "two\n2\n");
    assert_eq!(field(&tool(&db, &["verify"]), "records"), "2");
    assert_eq!(
        sorted(tool(&db, &["entries"])),
        ["alpha => one", "gamma => three"]
    );
}

#[test]
fn recover() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    let lines = tool(&db, &["recover"]);
    assert_eq!(field(&lines, "recovered-keys"), "2");
    assert_eq!(field(&lines, "failed-keys"), "0");
    assert!(!lines.iter().any(|l| l.starts_with("backup ")));

    let lines = tool(&db, &["recover", "--backup"]);
    let backup = Path::new(field(&lines, "backup")).to_path_buf();
    assert!(backup.exists());
    assert_eq!(
        sorted(tool(&backup, &["entries"])),
        ["alpha => one", "beta => two"]
    );
    assert_eq!(tool(&db, &["len"]), ["2"]);
}

#[test]
fn maintenance_needs_write_access() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());

    for command in ["recover", "reorganize", "upgrade", "downgrade"] {
        let output = run(GDBMTOOL, &["-r", db.to_str().unwrap(), command]);
        assert!(!output.status.success(), "{command}");
        assert!(stderr(&output).contains("read-only"), "{command}");
    }
}

// C GDBM's gdbm_load and gdbm_dump, if installed; required if REQUIRE_GDBM is set (as in CI)
fn c_gdbm_tools() -> bool {
    let found = Command::new("gdbm_load")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    assert!(
        found || std::env::var_os("REQUIRE_GDBM").is_none(),
        "REQUIRE_GDBM is set but C GDBM's gdbm_load was not found"
    );
    found
}

fn c_tool(bin: &str, args: &[&str]) {
    let output = run(bin, args);
    assert!(
        output.status.success(),
        "{bin} {args:?}: {}",
        stderr(&output)
    );
}

#[test]
fn c_gdbm_interop() {
    if !c_gdbm_tools() {
        eprintln!("skipping: C GDBM tools not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_string();
    let expected = ["alpha => one", "beta => two"];

    // our dumps load into C GDBM databases, which we read directly
    tool(&db, &["dump", &path("ours.ascii")]);
    tool(&db, &["export", &path("ours.bin")]);
    c_tool("gdbm_load", &[&path("ours.ascii"), &path("c-ascii.db")]);
    c_tool("gdbm_load", &[&path("ours.bin"), &path("c-bin.db")]);
    for c_db in ["c-ascii.db", "c-bin.db"] {
        assert_eq!(sorted(tool(Path::new(&path(c_db)), &["entries"])), expected);
        assert_eq!(
            field(&tool(Path::new(&path(c_db)), &["verify"]), "records"),
            "2"
        );
    }

    // C GDBM's dumps load into our databases
    c_tool("gdbm_dump", &[&path("c-ascii.db"), &path("c.ascii")]);
    c_tool(
        "gdbm_dump",
        &["--format=binary", &path("c-ascii.db"), &path("c.bin")],
    );
    let output = run(
        GDBMTOOL,
        &["-c", &path("from-ascii.db"), "load", &path("c.ascii")],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let output = run(
        GDBMTOOL,
        &["-c", &path("from-bin.db"), "import", &path("c.bin")],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    for our_db in ["from-ascii.db", "from-bin.db"] {
        assert_eq!(
            sorted(tool(Path::new(&path(our_db)), &["entries"])),
            expected
        );
    }
}
