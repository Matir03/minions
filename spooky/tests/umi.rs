use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_umi_go_simple() {
    let mut cmd = Command::cargo_bin("spooky").unwrap();
    cmd.arg("umi")
        .write_stdin("position fen 12|12 0|0 LLLLLLLLLLLLLLLLLLLLLLLLL|LLLLLLLLLLLLLLLLLLLLLLLLL f|I|i|||0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0 f|I|i|||0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0 0 1\ngo\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("turn").and(predicate::str::contains("endturn")));
}
