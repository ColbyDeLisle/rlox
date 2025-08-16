use std::fs;

fn main() {
    let contents = fs::read_to_string("lox/src/test.lox")
        .expect("Should have been able to read the file");

    dbg!(contents);
}