use std::collections::HashMap;
use anyhow::{Result, anyhow, Context};

use crate::ast::Expr;



pub struct Environment {
    pub env: EnvInner<Expr>,
}

impl Environment {
    pub fn new() -> Self {
        Self { env: EnvInner::new() }
    }

    pub fn push(&mut self) {
        self.env.push();
    }
    pub fn pop(&mut self) {
        self.env.pop();
    }
}


pub struct EnvInner<T: Clone> {
    map: Vec<HashMap<String, T>>,
}

impl<T: Clone> EnvInner<T> {
    fn new() -> Self {
        EnvInner { map: vec![HashMap::new()]  }
    }
    fn push(&mut self) {
        self.map.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.map.pop();
    }
    pub fn define(&mut self, name: String, val: T) -> Result<()> {
        let map = self.get_map_mut(self.depth())?;
        map.insert(name, val);
        Ok(())
    }
    pub fn get(&self, name: &String) -> Result<&T> {
        for level in (0..=self.depth()).rev() {
            if let Ok(lit) = self.get_level(level, name) {
                return Ok(lit);
            }
        }
        Err(anyhow!("Variable '{}' not defined.", name))
    }
    fn get_level(&self, level: usize, name: &String) -> Result<&T> {
        let map = self.get_map(level)?;

        match map.get(name) {
            Some(lit) => Ok(lit),
            None => Err(anyhow!("")) // error string will be overwritten in get()
        }
    }
    pub fn exists(&self, name: &String) -> bool {
        for level in (0..=self.depth()).rev() { 
            if self.exists_level(level, name) { return true; }
        }
        false
    }
    fn exists_level(&self, level: usize, name: &String) -> bool {
        self.get_map(level).unwrap()
            .contains_key(name)
    }
    pub fn assign(&mut self, name: &String, val: T) -> Result<()> {
        for level in (0..=self.depth()).rev() {
            if let Ok(_) = self.assign_level(level, name, val.clone()) {
                return Ok(());
            }
        }
        Err(anyhow!("Variable '{}' not defined.", name))
    }
    fn assign_level(&mut self, level: usize, name: &String, val: T) -> Result<()> {
        let lit = self.get_map_mut(level)?
            .get_mut(name).context(anyhow!(""))?;
        *lit = val;
        Ok(())
    }
    fn get_map(&self, level: usize) -> Result<&HashMap<String, T>> {
        self.map.get(level)
            .with_context(|| format!("Environment of depth {level} does not exist. This should never happen"))
    }
    fn get_map_mut(&mut self, level: usize) -> Result<&mut HashMap<String, T>> {
        self.map.get_mut(level)
            .with_context(|| format!("Environment of depth {level} does not exist. This should never happen"))
    }
    fn depth(&self) -> usize {
        self.map.len()-1
    }
}