use chumsky::Parser;

// mod parse;
mod parse2;
mod ast;
mod interpret;
mod env;


fn main() {
    let test = std::fs::read_to_string("/home/benb/workspace/rust/dhall2/dhall/test.dhall").unwrap();
    let dhall_text = std::fs::read_to_string("/home/benb/workspace/rust/dhall2/dhall/ex0.dhall").unwrap();

    // parse2
    let expr = parse2::dhall_parser()
        .parse(test);
    println!("{expr:#?}");

    // old
    // let expr = parse::program().parse(dhall_text.as_bytes());
    // // let expr = parse::test().parse(test.as_bytes());
    // println!("{expr:#?}");

    let result = interpret::interpret(&expr.unwrap());
    println!("{result:#?}");
}
