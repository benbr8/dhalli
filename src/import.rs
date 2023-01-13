
use std::{collections::BTreeMap, path::{Path, PathBuf}, cell::RefCell};
use anyhow::{Result, Context, anyhow};
use chumsky::Parser;

use crate::{ast::*, parse2::dhall_parser, interpret::interpret};


thread_local! {
    static IMPORT_LOCAL: RefCell<BTreeMap<PathBuf, Expr>> = RefCell::new(BTreeMap::new());
}




pub fn import_file_local(file: &PathBuf) -> Result<Expr> {
    let path = std::fs::canonicalize(file)?;

    let cache_entry = IMPORT_LOCAL.with(|map| {
        let map = map.borrow();
        map.get(&path).cloned()
    });

    if let Some(expr) = cache_entry {
        return Ok(expr);
    } else {
        let code = std::fs::read_to_string(&path)?;
        let parsed_expr = dhall_parser().parse(code)
            .map_err(|e| {
                anyhow!("Error parsing file {:?}: {:#?}", path, e)
            })?;
        let expr = interpret(&parsed_expr, &path)?;

        IMPORT_LOCAL.with(|map| map.borrow_mut().insert(path, expr.clone()));
        
        Ok(expr)
    }

}

