use crate::embedded::Bytecode;
use std::slice::Iter;

pub struct BytecodeIterator<'a> {
    iter: Iter<'a, Bytecode>,
    pub index: Option<usize>,
    code: &'a [Bytecode],
}

impl<'a> BytecodeIterator<'a> {
    pub fn new(code: &'a Vec<Bytecode>) -> BytecodeIterator<'a> {
        BytecodeIterator {
            iter: code.iter(),
            index: None,
            code,
        }
    }

    pub fn index(&self) -> usize {
        self.index.unwrap_or_else(|| 0)
    }

    pub fn as_slice(&self) -> &[Bytecode] {
        self.code
    }

    pub fn remaining_code(&self) -> &[Bytecode] {
        self.iter.as_slice()
    }

    pub fn absolute(&self, index: usize) -> &Bytecode {
        &self.code.get(index).unwrap_or_else(|| &Bytecode::Nop)
    }

    pub fn by_relative(&self, offset: isize) -> &Bytecode {
        &self
            .code
            .get(self.index() + offset as usize)
            .unwrap_or_else(|| &Bytecode::Nop)
    }
}

impl<'a> Iterator for BytecodeIterator<'a> {
    type Item = &'a Bytecode;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.index {
            self.index = Some(index + 1);
        } else {
            self.index = Some(0);
        }

        let item = self.iter.next();
        item
    }
}
