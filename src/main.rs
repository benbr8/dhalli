use std::path::PathBuf;

use chumsky::Parser;

// mod parse;
mod parse2;
mod ast;
mod interpret;
mod env;
mod import;


fn main() {
    let test = PathBuf::from("/home/benb/workspace/rust/dhall2/dhall/test.dhall");
    let dhall_text = PathBuf::from("/home/benb/workspace/rust/dhall2/dhall/ex0.dhall");

    // parse2
    let expr = import::import_file_local(&test);
    println!("{expr:?}");
}
