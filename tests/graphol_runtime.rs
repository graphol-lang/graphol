use graphol_rs::run_graphol;
use graphol_rs::runtime::{OutputMode, RuntimeIo, TestIo};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

fn values(source: &str, inputs: Vec<&str>) -> Vec<String> {
    let io = TestIo::new(inputs.into_iter().map(ToString::to_string).collect());
    let events = run_graphol(source, Box::new(io)).expect("program should run");
    events.into_iter().map(|event| event.value).collect()
}

#[derive(Default)]
struct InputCapture {
    prompts: Vec<String>,
}

struct CapturingIo {
    inputs: VecDeque<String>,
    capture: Rc<RefCell<InputCapture>>,
}

impl RuntimeIo for CapturingIo {
    fn read_input(&mut self, prompt: &str) -> String {
        self.capture.borrow_mut().prompts.push(prompt.to_string());
        self.inputs.pop_front().unwrap_or_default()
    }

    fn on_output(&mut self, _mode: OutputMode, _value: &str) {}
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

    assert_eq!(values(source, vec![]), vec!["4", "6", "9", "12", "4"]);
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

    assert_eq!(values(source, vec![]), vec!["the double is:", "10"]);
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

    assert_eq!(
        values(source, vec![]),
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

    let out = values(source, vec![]);
    assert_eq!(out.len(), 24);
    assert!(out.contains(&"FOOOOOOOOO".to_string()));
    assert!(out.contains(&"BARRRRRRRR".to_string()));
    assert!(out.contains(&"BAZZZZZZZZ".to_string()));
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

    let capture = Rc::new(RefCell::new(InputCapture::default()));
    let io = CapturingIo {
        inputs: vec!["Chavao".to_string(), "12".to_string()].into(),
        capture: capture.clone(),
    };

    let events = run_graphol(source, Box::new(io)).expect("program should run");
    let out: Vec<String> = events.into_iter().map(|event| event.value).collect();

    assert_eq!(out, vec!["the double is:", "24"]);
    assert_eq!(
        capture.borrow().prompts.clone(),
        vec![
            "What is your name?".to_string(),
            "Hello Chavao, tell me a number.".to_string()
        ]
    );
}
