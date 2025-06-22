use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_umi_go_simple() {
    let mut cmd = Command::cargo_bin("spooky").unwrap();
    cmd.arg("umi")
        .write_stdin("position fen 2 2 0,1 4 1,2,3,4 N8z/0/0/0/0/0/0/0/0/0|0/0/0/0/0/0/0/0/0/Z8n 0 LLLU|LLLA 10|5\ngo\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("turn").and(predicate::str::contains("endturn")));
}
