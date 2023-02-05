use std::{collections::BTreeMap, path::{Path, PathBuf}, cell::RefCell};

use chumsky::Parser;

use crate::{bytecode::Function, error::CompileError, compiler, parse2};

thread_local! {
    static IMPORT_LOCAL: RefCell<BTreeMap<String, Function>> = RefCell::new(BTreeMap::new());
}


pub fn import_file_local(path: &PathBuf) -> Result<Function, CompileError> {
    let path_string = path.to_string_lossy().to_string();

    let cache_entry = IMPORT_LOCAL.with(|map| {
        let map = map.borrow();
        map.get(&path_string).cloned()
    });


    if let Some(func) = cache_entry {
        Ok(func)
    } else {
        println!("Importing file {path_string}.");
        let code = std::fs::read_to_string(path).unwrap();
        let ast = parse2::dhall_parser().parse(code).unwrap();

        let func = compiler::compile(&ast, path.clone()).unwrap();

        IMPORT_LOCAL.with(|map|
            map.borrow_mut()
            .insert(path_string, func.clone()));

        Ok(func)
    }
}