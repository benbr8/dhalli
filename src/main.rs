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





// fn main2() {
//     let test = std::fs::read_to_string("/home/benb/workspace/rust/dhall2/dhall/test.dhall").unwrap();
//     let dhall_text = std::fs::read_to_string("/home/benb/workspace/rust/dhall2/dhall/ex0.dhall").unwrap();

//     // parse2
//     let expr = parse2::dhall_parser()
//         .parse(test);
//     println!("{expr:#?}");

//     // old
//     // let expr = parse::program().parse(dhall_text.as_bytes());
//     // // let expr = parse::test().parse(test.as_bytes());
//     // println!("{expr:#?}");

//     let result = interpret::interpret(&expr.unwrap());
//     println!("{result:#?}");
// }
