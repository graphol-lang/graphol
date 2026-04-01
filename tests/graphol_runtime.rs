use graphol::compile_entry_to_binary;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn compile_and_run(main_source: &str, extra_files: &[(&str, &str)], inputs: &[&str]) -> String {
    let root = create_temp_dir("native_runtime");
    fs::write(root.join("main.graphol"), main_source).expect("main.graphol should be written");

    for (relative_path, content) in extra_files {
        fs::write(root.join(relative_path), content).expect("extra source should be written");
    }

    let output_path = root.join("program");
    compile_entry_to_binary(&root.join("main.graphol"), &output_path)
        .expect("native compilation should succeed");

    let mut child = Command::new(&output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled program should spawn");

    if !inputs.is_empty() {
        let payload = format!("{}\n", inputs.join("\n"));
        child
            .stdin
            .as_mut()
            .expect("stdin should be available")
            .write_all(payload.as_bytes())
            .expect("stdin payload should be written");
    }

    let output = child
        .wait_with_output()
        .expect("program execution should finish");

    assert!(
        output.status.success(),
        "compiled program failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn output_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[test]
fn executes_numeric_operations_demo() {
    let source = r#"
echo (2 + 2)
echo (+ 3 3)
echo (+ 2 3 4)
echo (8 + 3 4 - 1 2)
echo (8 * 2 / 4)
"#;

    let stdout = compile_and_run(source, &[], &[]);
    assert_eq!(output_lines(&stdout), vec!["4", "6", "9", "12", "4"]);
}

#[test]
fn executes_blocks_with_inbox() {
    let source = r#"
double {
   x inbox
   echo "the double is:" (x * 2)
}

number 5
double number run
"#;

    let stdout = compile_and_run(source, &[], &[]);
    assert_eq!(output_lines(&stdout), vec!["the double is:", "10"]);
}

#[test]
fn executes_conditionals_and_else() {
    let source = r#"
if (= 3 6){
      echo "This is false"
 } (> 4 5) {
      echo "This is not true"
 } else {
      echo "Actually, no truths around here..."
}

if (!(= 3 6)){
      echo "In programming, the negation of a lie is truth!"
 } (< 4 5) {
      echo "And this is also true"
 } else {
      echo "Over here, only truths..."
}

if (x| (= 6 6) (= 3 6) ) {
      echo "Either one thing or another!"
 } (x| (= 6 6) (= 3 3) ) {
      echo "Both, no way!"
 } (x| (= 6 3) (= 3 6) ) {
      echo "Neither, not even close!"
 }
"#;

    let stdout = compile_and_run(source, &[], &[]);
    assert_eq!(
        output_lines(&stdout),
        vec![
            "Actually, no truths around here...",
            "In programming, the negation of a lie is truth!",
            "And this is also true",
            "Either one thing or another!"
        ]
    );
}

#[test]
fn executes_async_blocks_with_all_outputs() {
    let source = r#"
stdout "console"
foo {
    echo "FOO"
    echo "FOOO"
    echo "FOOOO"
    echo "FOOOOO"
    echo "FOOOOOO"
    echo "FOOOOOOO"
    echo "FOOOOOOOO"
    echo "FOOOOOOOOO"
}

bar {
    echo "BAR"
    echo "BARR"
    echo "BARRR"
    echo "BARRRR"
    echo "BARRRRR"
    echo "BARRRRRR"
    echo "BARRRRRRR"
    echo "BARRRRRRRR"
}

baz {
    echo "BAZ"
    echo "BZZZ"
    echo "BAZZZ"
    echo "BAZZZZ"
    echo "BAZZZZZ"
    echo "BAZZZZZZ"
    echo "BAZZZZZZZ"
    echo "BAZZZZZZZZ"
}

foo async run
bar async run
baz run
"#;

    let stdout = compile_and_run(source, &[], &[]);
    let lines = output_lines(&stdout);
    assert_eq!(lines.len(), 24);
    assert!(lines.contains(&"FOOOOOOOOO".to_string()));
    assert!(lines.contains(&"BARRRRRRRR".to_string()));
    assert!(lines.contains(&"BAZZZZZZZZ".to_string()));
}

#[test]
fn composes_input_prompt_and_reads_once_per_expression() {
    let source = r#"
double {
   x inbox
   echo "the double is:" (x * 2)
}

name (input "What is your name?")
number 0 (input "Hello " name ", tell me a number.")
double number run
"#;

    let stdout = compile_and_run(source, &[], &["Chavao", "12"]);
    assert!(stdout.contains("What is your name?"));
    assert!(stdout.contains("Hello Chavao, tell me a number."));
    assert!(stdout.contains("the double is:"));
    assert!(stdout.contains("24"));
}

#[test]
fn executes_program_with_include_from_file() {
    let main = "include \"program3.graphol\"\n\ndouble 44 run\n";
    let program3 = "double {\n   x inbox\n   echo \"the double is:\" (x * 2)\n}\n\nname (input \"What is your name?\")\nnumber 0 (input \"Hello \" name \", tell me a number.\")\ndouble number run\n";

    let stdout = compile_and_run(main, &[("program3.graphol", program3)], &["Ada", "44"]);
    assert_eq!(
        output_lines(&stdout),
        vec![
            "What is your name? Hello Ada, tell me a number. the double is:",
            "88",
            "the double is:",
            "88"
        ]
    );
}

fn create_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("graphol_native_test_{}_{}", name, timestamp));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}
