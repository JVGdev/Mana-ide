//! A program as the machine runs it: assembled, decoded once, and how long each of its orders is, worked out once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::assembler::Program;
use super::disassembler::{DecodeError, Instr, decode, decode_all};

#[derive(Debug)]
pub struct Code {
    pub program: Program,
    /// Every instruction, by the address it starts at.
    decoded: Vec<Option<Instr>>,
    /// How long the order at each address is (`order_length`), once it's been asked.
    lengths: Mutex<HashMap<usize, usize>>,
}

/// Why an instruction couldn't be fetched: the mind ran off the end of the spell, or found bytes that aren't one.
#[derive(Debug, Clone)]
pub enum FetchError {
    OffTheEnd(usize),
    Decode(DecodeError),
}

impl Code {
    pub fn new(program: Program) -> Arc<Code> {
        let mut decoded = vec![None; program.bytes.len()];
        if let Ok(all) = decode_all(&program.bytes) {
            for i in all {
                decoded[i.addr] = Some(i);
            }
        }
        Arc::new(Code { program, decoded, lengths: Mutex::new(HashMap::new()) })
    }

    pub fn len(&self) -> usize {
        self.program.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.program.bytes.is_empty()
    }

    /// The instruction at `addr`.
    pub fn fetch(&self, addr: usize) -> Result<Instr, FetchError> {
        if addr >= self.program.bytes.len() {
            return Err(FetchError::OffTheEnd(addr));
        }
        match self.decoded[addr] {
            Some(i) => Ok(i),
            // Not where an instruction starts, as the assembler laid them out: decode what's there.
            None => decode(&self.program.bytes, addr).map_err(FetchError::Decode),
        }
    }

    /// How long an order is: every instruction it could ever run from `addr`, following its jumps and calls. Ingraining
    /// it into a particle takes a beat for each.
    pub fn order_length(&self, addr: usize) -> usize {
        if let Some(&n) = self.lengths.lock().unwrap().get(&addr) {
            return n;
        }
        let mut seen = std::collections::HashSet::new();
        let mut todo = vec![addr];
        while let Some(a) = todo.pop() {
            if seen.contains(&a) || a >= self.program.bytes.len() {
                continue;
            }
            let Ok(instr) = self.fetch(a) else { continue };
            seen.insert(a);
            let name = instr.op.name;
            let next = a + instr.size;
            if ["RET", "HALT", "DISS", "FAIL", "TICK"].contains(&name) {
                continue;
            }
            if name == "JMP" {
                todo.push(instr.args[0] as usize);
            } else if name == "CALL" || name.starts_with('J') {
                todo.push(instr.args[0] as usize);
                todo.push(next);
            } else {
                todo.push(next);
            }
        }
        self.lengths.lock().unwrap().insert(addr, seen.len());
        seen.len()
    }
}
