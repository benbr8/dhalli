use std::path::PathBuf;
use chumsky::Parser;


mod parse2;
mod ast;
// mod interpret;
// mod env;
// mod import;
mod import2;
mod naive_double;
mod bytecode;
mod vm;
mod compiler;
mod error;


fn main() {
    let filename = std::env::args().nth(1).expect("no file name given");
    let path = std::fs::canonicalize(PathBuf::from(filename)).unwrap();
    let code = std::fs::read_to_string(&path).unwrap();
    let ast = parse2::dhall_parser().parse(code).unwrap();
    println!("AST:");
    println!("{:?}", &ast);

    let function = compiler::compile(&ast, path);
    println!("Function:");
    println!("{:?}", &function);

    let r = vm::run_function(function.unwrap(), true);
    println!("{r:#?}");
}


// fn main2() {

//     let filename = std::env::args().nth(1).expect("no file name given");

//     let dhall_text = PathBuf::from(filename);

//     // parse2
//     let expr = import::parse_and_interpret(&dhall_text);
//     if let Err(e) = &expr {
//         // let re = Regex::new(r#"(?s)Error parsing file "(.*?)".*span: (\d+?)\.\.(\d+?),"#).unwrap();
//         let err_str = format!("{:?}", e);
//         let re_file = Regex::new(r#"Error parsing file "(.*?)""#).unwrap();
//         let mut file_captures = re_file.captures_iter(&err_str);
//         let mut file_capture = file_captures.next();
//         for cap in file_captures {
//             file_capture = Some(cap);
//         }

//         if let Some(cap) = file_capture {
//             let path = cap.get(1).unwrap().as_str();

//             let re_range = Regex::new(r#"span: (\d+?)\.\.(\d+?),"#).unwrap();
//             if let Some(caps) = re_range.captures(&err_str) {
//                 let mut li = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
//                 let mut ri = caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
//                 let code = std::fs::read_to_string(path).unwrap();
//                 if !(li < 20) { li -= 20 } else { li = 0 }
//                 if !(ri + 20 > code.len()) { ri += 20 } else { ri = code.len() }
//                 let slice = &code.as_str()[li..ri];
//                 println!("{slice}");

//             }

//         }
//         // if let Some(caps) = re.captures(&format!("{e:?}")) {
//         //     // println!()
//         //     let path = caps.get(1).unwrap().as_str();
//         //     let mut li = caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
//         //     let mut ri = caps.get(3).unwrap().as_str().parse::<usize>().unwrap();
//         //     let code = std::fs::read_to_string(path).unwrap();
//         //     // let code = std::fs::read_to_string(path).unwrap().split_at(li-10).1.to_string();
//         //     // let code = code.split_at(ri-li+10).0.to_string();
//         //     if !(li < 10) { li -= 10 } else { li = 0 }
//         //     if !(ri + 10 > code.len()) { ri += 10 } else { ri = code.len() }
//         //     let slice = &code.as_str()[li..ri];
//         //     println!("{slice}");
//         // }
//     }
//     println!("{expr:?}");
// }
