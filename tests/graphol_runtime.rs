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
dobra {
   x inbox
   echo "o dobro e:" (x * 2)
}

numero 5
dobra numero run
"#;

    assert_eq!(values(source, vec![]), vec!["o dobro e:", "10"]);
}

#[test]
fn executes_conditionals_and_else() {
    let source = r#"
if (= 3 6){
      echo "Isto e falso"
 } (> 4 5) {
      echo "Isto nao e verdade"
 } else {
      echo "Na verdade, sem verdades por aqui..."
}

if (!(= 3 6)){
      echo "Em programacao, a negacao de uma mentira e uma verdade!"
 } (< 4 5) {
      echo "E isto tambem e verdade"
 } else {
      echo "Ja por aqui, so verdades..."
}

if (x| (= 6 6) (= 3 6) ) {
      echo "Ou uma coisa ou outra!"
 } (x| (= 6 6) (= 3 3) ) {
      echo "As duas, nem pensar!"
 } (x| (= 6 3) (= 3 6) ) {
      echo "Nenhuma, muito menos!"
 }
"#;

    assert_eq!(
        values(source, vec![]),
        vec![
            "Na verdade, sem verdades por aqui...",
            "Em programacao, a negacao de uma mentira e uma verdade!",
            "E isto tambem e verdade",
            "Ou uma coisa ou outra!"
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
dobra {
   x inbox
   echo "o dobro e:" (x * 2)
}

nome (input "Qual o seu nome?")
numero 0 (input "Ola " nome ", diga um numero.")
dobra numero run
"#;

    let capture = Rc::new(RefCell::new(InputCapture::default()));
    let io = CapturingIo {
        inputs: vec!["Chavao".to_string(), "12".to_string()].into(),
        capture: capture.clone(),
    };

    let events = run_graphol(source, Box::new(io)).expect("program should run");
    let out: Vec<String> = events.into_iter().map(|event| event.value).collect();

    assert_eq!(out, vec!["o dobro e:", "24"]);
    assert_eq!(
        capture.borrow().prompts.clone(),
        vec![
            "Qual o seu nome?".to_string(),
            "Ola Chavao, diga um numero.".to_string()
        ]
    );
}
