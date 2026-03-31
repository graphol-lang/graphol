use graphol_rs::source_loader::{load_entry_source, resolve_source};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn resolves_nested_includes_relative_to_current_file() {
    let root = create_temp_dir("nested_includes");
    let sub = root.join("sub");
    fs::create_dir_all(&sub).expect("subdir should be created");

    fs::write(
        root.join("main.graphol"),
        "include \"sub/lib.graphol\"\necho \"main\"\n",
    )
    .expect("main should be written");
    fs::write(
        sub.join("lib.graphol"),
        "include \"../shared.graphol\"\necho \"lib\"\n",
    )
    .expect("lib should be written");
    fs::write(root.join("shared.graphol"), "echo \"shared\"\n").expect("shared should be written");

    let resolved = load_entry_source(&root.join("main.graphol")).expect("include should resolve");
    assert_eq!(resolved, "echo \"shared\"\necho \"lib\"\necho \"main\"\n");
}

#[test]
fn includes_same_file_only_once() {
    let root = create_temp_dir("include_once");
    fs::write(
        root.join("main.graphol"),
        "include \"shared.graphol\"\ninclude \"shared.graphol\"\necho \"end\"\n",
    )
    .expect("main should be written");
    fs::write(root.join("shared.graphol"), "echo \"shared\"\n").expect("shared should be written");

    let resolved = load_entry_source(&root.join("main.graphol")).expect("include should resolve");
    assert_eq!(resolved, "echo \"shared\"\necho \"end\"\n");
}

#[test]
fn loads_main_graphol_when_entry_path_is_directory() {
    let root = create_temp_dir("dir_entry_defaults_to_main");
    fs::write(root.join("main.graphol"), "echo \"main\"\n").expect("main should be written");

    let resolved = load_entry_source(&root).expect("directory entry should load main.graphol");
    assert_eq!(resolved, "echo \"main\"\n");
}

#[test]
fn fails_on_include_cycle() {
    let root = create_temp_dir("include_cycle");
    fs::write(root.join("a.graphol"), "include \"b.graphol\"\n").expect("a should be written");
    fs::write(root.join("b.graphol"), "include \"a.graphol\"\n").expect("b should be written");

    let error = load_entry_source(&root.join("a.graphol")).expect_err("cycle should fail");
    assert!(error.message.contains("include cycle detected"));
}

#[test]
fn source_without_base_rejects_include() {
    let error = resolve_source("include \"foo.graphol\"\n", None)
        .expect_err("include without base should fail");
    assert!(error.message.contains("file-based execution context"));
}

#[test]
fn include_keyword_is_reserved() {
    let error = resolve_source("value include other\n", Some(Path::new(".")))
        .expect_err("reserved keyword should fail");
    assert!(error.message.contains("reserved keyword"));
}

fn create_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("graphol_include_test_{}_{}", name, timestamp));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
