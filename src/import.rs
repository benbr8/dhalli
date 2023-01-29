
use std::{collections::BTreeMap, path::{Path, PathBuf}, cell::RefCell};
use anyhow::{Result, Context, anyhow};
use chumsky::Parser;

use crate::{ast::*, parse2::dhall_parser, interpret::interpret};


thread_local! {
    static IMPORT_LOCAL: RefCell<BTreeMap<String, Expr>> = RefCell::new(BTreeMap::new());
}




pub fn import_file_local(file: &PathBuf) -> Result<Expr> {
    let path = std::fs::canonicalize(file)?;
    let path_string = path.to_string_lossy().to_string();

    let cache_entry = IMPORT_LOCAL.with(|map| {
        let map = map.borrow();
        map.get(&path_string).cloned()
    });

    if let Some(expr) = cache_entry {
        return Ok(expr);
    } else {
        let code = std::fs::read_to_string(&path)?;
        let expr = parse(code)
            .with_context(|| format!("Error parsing file {:?}", path))?;

        IMPORT_LOCAL.with(|map| map.borrow_mut().insert(path_string, expr.clone()));

        Ok(expr)
    }

}

pub fn import_env(env: String, file: &PathBuf) -> Result<Expr> {
    let path = std::fs::canonicalize(file)?;

    let cache_entry = IMPORT_LOCAL.with(|map| {
        let map = map.borrow();
        map.get(&env).cloned()
    });

    if let Some(expr) = cache_entry {
        return Ok(expr);
    } else {
        let code = std::env::var(&env)?;
        let expr = parse(code)
            .with_context(|| format!("Error parsing code from environment variable {:?}", &env))?;

        IMPORT_LOCAL.with(|map| map.borrow_mut().insert(env, expr.clone()));

        Ok(expr)
    }

}


fn parse(code: String) -> Result<Expr> {
    dhall_parser().parse(code)
        .map_err(|e| {
            anyhow!("{:#?}", e)
        })

}

pub fn parse_and_interpret(file: &PathBuf) -> Result<Expr> {
    let path = std::fs::canonicalize(file)?;
    let code = std::fs::read_to_string(&path)?;
    let parsed_expr = parse(code)?;
    // println!("{:#?}", &parsed_expr);
    let expr = interpret(&parsed_expr, &file)?;

    Ok(expr)
}