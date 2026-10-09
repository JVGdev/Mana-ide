//! Bytes → instructions. The machine runs what this decodes, and the tester prints it.

use std::collections::HashMap;

use indexmap::IndexMap;

use super::isa::{IMMEDIATE_BIT, Kind, LOCK_NAMES, MANA_BIT, Op, op_by_code, port_by_code};
use crate::js;

#[derive(Debug, Clone, Copy)]
pub struct Instr {
    pub addr: usize,
    pub size: usize,
    pub op: &'static Op,
    /// One number per operand: a register index, an address, a port or constant code, or an immediate's value.
    pub args: [f64; 4],
    /// The last operand is an immediate, not a register.
    pub immediate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeError(pub String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DecodeError {}

pub fn decode(bytes: &[u8], addr: usize) -> Result<Instr, DecodeError> {
    let Some(&first) = bytes.get(addr) else {
        return Err(DecodeError(format!("no instruction at {}", hex(addr, 4))));
    };
    let immediate = first & IMMEDIATE_BIT != 0;
    let Some(op) = op_by_code(first & !IMMEDIATE_BIT) else {
        return Err(DecodeError(format!("no instruction {} at {}", hex(first as usize, 2), hex(addr, 4))));
    };
    let short = || DecodeError(format!("{} at {} runs off the end", op.name, hex(addr, 4)));
    let mut args = [0.0; 4];
    let mut at = addr + 1;
    let n = op.operands.len();
    for (k, kind) in op.operands.iter().enumerate() {
        let last = k == n - 1;
        if *kind == Kind::L {
            let b = bytes.get(at..at + 2).ok_or_else(short)?;
            args[k] = u16::from_le_bytes([b[0], b[1]]) as f64;
            at += 2;
        } else if *kind == Kind::I || (*kind == Kind::S && last && immediate) {
            let b = bytes.get(at..at + 4).ok_or_else(short)?;
            args[k] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64;
            at += 4;
        } else if *kind == Kind::M {
            args[k] = (bytes.get(at).ok_or_else(short)? & !MANA_BIT) as f64;
            at += 1;
        } else {
            args[k] = *bytes.get(at).ok_or_else(short)? as f64;
            at += 1;
        }
    }
    Ok(Instr { addr, size: at - addr, op, args, immediate })
}

/// Decodes a whole program, in order.
pub fn decode_all(bytes: &[u8]) -> Result<Vec<Instr>, DecodeError> {
    let mut out = Vec::new();
    let mut addr = 0;
    while addr < bytes.len() {
        let instr = decode(bytes, addr)?;
        addr += instr.size;
        out.push(instr);
    }
    Ok(out)
}

/// One instruction as text. Labels, when given, name jump targets.
pub fn format(instr: &Instr, labels: Option<&HashMap<usize, String>>) -> String {
    let n = instr.op.operands.len();
    let parts: Vec<String> = instr
        .op
        .operands
        .iter()
        .enumerate()
        .map(|(k, kind)| {
            let v = instr.args[k];
            let r = v as i64;
            let last = k == n - 1;
            match kind {
                Kind::N => {
                    // IN reads a size-3 port into a triple.
                    let triple = instr.op.name == "IN" && port_by_code(instr.args[1] as u8).is_some_and(|p| p.size == 3);
                    if triple { format!("n{r}:{}", r + 2) } else { format!("n{r}") }
                }
                Kind::M => format!("m{r}"),
                Kind::T => format!("n{r}:{}", r + 2),
                Kind::A => format!("[n{r}]"),
                Kind::S => {
                    if last && instr.immediate {
                        format!("#{}", num(v))
                    } else {
                        format!("n{r}")
                    }
                }
                Kind::I => format!("#{}", num(v)),
                Kind::K => {
                    if instr.op.name == "LOCK" {
                        LOCK_NAMES.get(r as usize).map(|s| s.to_string()).unwrap_or_else(|| format!("#{r}"))
                    } else {
                        format!("#{r}")
                    }
                }
                Kind::P => port_by_code(r as u8).map(|p| p.name.to_string()).unwrap_or_else(|| format!("?{r}")),
                Kind::L => match labels.and_then(|l| l.get(&(r as usize))) {
                    Some(name) => match name.find('.') {
                        Some(i) => name[i..].to_string(),
                        None => name.clone(),
                    },
                    None => hex(r as usize, 4),
                },
            }
        })
        .collect();
    if parts.is_empty() { instr.op.name.to_string() } else { format!("{:<5} {}", instr.op.name, parts.join(", ")) }
}

/// A listing: address, bytes, instruction.
pub fn listing(bytes: &[u8], labels: Option<&IndexMap<String, usize>>) -> Result<String, DecodeError> {
    let mut by_addr: HashMap<usize, String> = HashMap::new();
    for (name, &addr) in labels.into_iter().flatten() {
        // Prefer the global name when two labels share an address.
        let had = by_addr.get(&addr);
        if had.is_none_or(|had| had.contains('.') && !name.contains('.')) {
            by_addr.insert(addr, name.clone());
        }
    }
    let mut out: Vec<String> = Vec::new();
    for instr in decode_all(bytes)? {
        let label = by_addr.get(&instr.addr);
        if let Some(label) = label
            && !label.contains('.')
        {
            out.push(format!("{label}:"));
        }
        let raw: Vec<String> = bytes[instr.addr..instr.addr + instr.size].iter().map(|b| hex(*b as usize, 2)).collect();
        let local = match label.and_then(|l| l.find('.').map(|i| &l[i..])) {
            Some(l) => format!("{l}:"),
            None => String::new(),
        };
        out.push(format!("{}  {:<23} {:<9}{}", hex(instr.addr, 4), raw.join(" "), local, format(&instr, Some(&by_addr))));
    }
    Ok(out.join("\n"))
}

fn hex(v: usize, width: usize) -> String {
    format!("{v:0width$X}")
}

fn num(v: f64) -> String {
    js::num(js::round(v * 1e5) / 1e5)
}
