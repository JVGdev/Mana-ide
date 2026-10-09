//! mas: .masm → bytes.
//!
//! ```text
//!   label:   INSTR  operand, operand   ; comment
//!   .local:  …                         a label scoped to the last global label
//!   .use     Elements, Shapes          take in libraries (each once, after the spell)
//!   .const   EARTH 3                   a name for a number, written #EARTH
//! ```

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;

use super::isa::{IMMEDIATE_BIT, Kind, MANA_BIT, Op, lock, op_by_name, port_by_name};

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLine {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub bytes: Vec<u8>,
    /// Global and local labels (`Fireball`, `Fireball.order`) to addresses, in the order they were defined.
    pub labels: IndexMap<String, usize>,
    pub consts: IndexMap<String, f64>,
    /// Where each instruction came from, by address.
    pub lines: BTreeMap<usize, SourceLine>,
}

#[derive(Debug, Clone)]
pub struct AsmError {
    pub problems: Vec<String>,
}

impl std::fmt::Display for AsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.problems.join("\n"))
    }
}

impl std::error::Error for AsmError {}

struct Pending {
    op: &'static Op,
    tokens: Vec<String>,
    scope: String,
    at: SourceLine,
    addr: usize,
}

static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\.?[A-Za-z_][A-Za-z0-9_]*):\s*").unwrap());
static CONST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\s+(-?[0-9.]+)$").unwrap());
static TRIPLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^n([0-9]+):([0-9]+)$").unwrap());
static ADDRESS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\[\s*n([0-9]+)\s*\]$").unwrap());
static N_REG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^n([0-9]+)(?::([0-9]+))?$").unwrap());
static M_REG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^m([0-9]+)(?::([0-9]+))?$").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[0-9.]+(e-?[0-9]+)?$").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// JavaScript's `Number(text)` for the digits and points the patterns above let through: NaN when it isn't a number.
fn number(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
}

/// Assembles a spell and the libraries it uses. `libraries` finds a library's source by name (`Shapes` → the text of
/// Shapes.masm).
pub fn assemble(source: &str, file: &str, libraries: &dyn Fn(&str) -> Option<String>) -> Result<Program, AsmError> {
    let mut problems: Vec<String> = Vec::new();
    let mut labels: IndexMap<String, usize> = IndexMap::new();
    let mut consts: IndexMap<String, f64> = IndexMap::new();
    let mut pending: Vec<Pending> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, String)> = VecDeque::from([(file.to_string(), source.to_string())]);
    let mut addr = 0usize;

    let fail = |problems: &mut Vec<String>, at: &SourceLine, message: String| {
        problems.push(format!("{}:{}: {}", at.file, at.line, message))
    };

    // Pass 1: read every file, place labels, size every instruction.
    while let Some((unit_file, unit_source)) = queue.pop_front() {
        let mut scope = String::new();
        for (i, raw) in split_lines(&unit_source).into_iter().enumerate() {
            let at = SourceLine { file: unit_file.clone(), line: i + 1, text: raw.trim().to_string() };
            let mut text: &str = match raw.find(';') {
                Some(k) => &raw[..k],
                None => raw,
            };
            text = text.trim();
            if text.is_empty() {
                continue;
            }

            // Labels, any number of them, at the start of the line.
            while let Some(m) = LABEL.captures(text) {
                let name = m.get(1).unwrap().as_str();
                let mut full = name.to_string();
                if name.starts_with('.') {
                    if scope.is_empty() {
                        fail(&mut problems, &at, format!("local label {name} before any global label"));
                    }
                    full = format!("{scope}{name}");
                } else {
                    scope = name.to_string();
                }
                if labels.contains_key(&full) {
                    fail(&mut problems, &at, format!("label {full} is defined twice"));
                }
                labels.insert(full, addr);
                text = &text[m.get(0).unwrap().end()..];
            }
            if text.is_empty() {
                continue;
            }

            if text.starts_with('.') {
                let words: Vec<&str> = SPACES.split(text).collect();
                let directive = words[0];
                let args = words[1..].join(" ");
                if directive == ".use" {
                    for lib in args.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        if used.contains(lib) {
                            continue;
                        }
                        used.insert(lib.to_string());
                        match libraries(lib) {
                            None => fail(&mut problems, &at, format!("no library called {lib}")),
                            Some(src) => queue.push_back((format!("{lib}.masm"), src)),
                        }
                    }
                } else if directive == ".const" {
                    match CONST.captures(&args) {
                        None => fail(&mut problems, &at, ".const needs a name and a number".into()),
                        Some(m) => {
                            let name = m.get(1).unwrap().as_str();
                            if consts.contains_key(name) {
                                fail(&mut problems, &at, format!("{name} is defined twice"));
                            } else {
                                consts.insert(name.to_string(), number(m.get(2).unwrap().as_str()));
                            }
                        }
                    }
                } else {
                    fail(&mut problems, &at, format!("unknown directive {directive}"));
                }
                continue;
            }

            let (mnemonic, rest) = match text.find(char::is_whitespace) {
                Some(k) => (text[..k].to_uppercase(), text[k..].trim()),
                None => (text.to_uppercase(), ""),
            };
            let Some(op) = op_by_name(&mnemonic) else {
                fail(&mut problems, &at, format!("unknown instruction {mnemonic}"));
                continue;
            };
            let tokens: Vec<String> =
                if rest.is_empty() { vec![] } else { rest.split(',').map(|s| s.trim().to_string()).collect() };
            if tokens.len() != op.operands.len() {
                fail(&mut problems, &at, format!("{} takes {} operand(s), not {}", op.name, op.operands.len(), tokens.len()));
                continue;
            }
            let mut size = 1;
            for (k, kind) in op.operands.iter().enumerate() {
                size += operand_size(*kind, &tokens[k]);
            }
            pending.push(Pending { op, tokens, scope: scope.clone(), at, addr });
            addr += size;
        }
    }

    // Pass 2: encode.
    let mut bytes = vec![0u8; addr];
    let mut lines = BTreeMap::new();
    for p in &pending {
        lines.insert(p.addr, p.at.clone());
        let mut at = p.addr + 1;
        let mut immediate = false;
        for (k, kind) in p.op.operands.iter().enumerate() {
            let token = p.tokens[k].as_str();
            let bad = |problems: &mut Vec<String>, what: &str| {
                fail(problems, &p.at, format!("{} operand {}: {}, not \"{}\"", p.op.name, k + 1, what, token))
            };
            match kind {
                Kind::N => {
                    match register(token, &N_REG, 32) {
                        None => bad(&mut problems, "a number register (n0–n31)"),
                        Some(r) => bytes[at] = r,
                    }
                    at += 1;
                }
                Kind::M => {
                    match register(token, &M_REG, 8) {
                        None => bad(&mut problems, "a mana register (m0–m7)"),
                        Some(r) => bytes[at] = MANA_BIT | r,
                    }
                    at += 1;
                }
                Kind::T => {
                    let m = TRIPLE.captures(token);
                    let a = m.as_ref().map(|m| number(m.get(1).unwrap().as_str())).unwrap_or(f64::NAN);
                    match &m {
                        Some(m) if number(m.get(2).unwrap().as_str()) == a + 2.0 && a <= 29.0 => bytes[at] = a as u8,
                        _ => bad(&mut problems, "three number registers, like n4:6"),
                    }
                    at += 1;
                }
                Kind::A => {
                    match ADDRESS.captures(token) {
                        Some(m) if number(m.get(1).unwrap().as_str()) <= 31.0 => {
                            bytes[at] = number(m.get(1).unwrap().as_str()) as u8
                        }
                        _ => bad(&mut problems, "an address in a register, like [n3]"),
                    }
                    at += 1;
                }
                Kind::S => {
                    if token.starts_with('#') {
                        match constant(token, &consts) {
                            None => bad(&mut problems, "a number or a known #NAME"),
                            Some(v) => bytes[at..at + 4].copy_from_slice(&(v as f32).to_le_bytes()),
                        }
                        immediate = true;
                        at += 4;
                    } else {
                        match register(token, &N_REG, 32) {
                            None => bad(&mut problems, "a number register or an immediate"),
                            Some(r) => bytes[at] = r,
                        }
                        at += 1;
                    }
                }
                Kind::I => {
                    let v = if token.starts_with('#') { constant(token, &consts) } else { None };
                    match v {
                        None => bad(&mut problems, "an immediate, like #2.5"),
                        Some(v) => bytes[at..at + 4].copy_from_slice(&(v as f32).to_le_bytes()),
                    }
                    immediate = true;
                    at += 4;
                }
                Kind::K => {
                    let v = if token.starts_with('#') {
                        constant(token, &consts)
                    } else {
                        lock(&token.to_uppercase()).map(f64::from)
                    };
                    if token.to_uppercase() == "SHAPE" {
                        bad(&mut problems, "INPUT or ORDER: a shape isn't locked any more, it's held by pushing (SPEC §11)");
                    } else {
                        match v {
                            Some(v) if v.fract() == 0.0 && (0.0..=255.0).contains(&v) => bytes[at] = v as u8,
                            _ => bad(&mut problems, "a small constant, like #3 or INPUT"),
                        }
                    }
                    at += 1;
                }
                Kind::P => {
                    match port_by_name(&token.to_uppercase()) {
                        None => bad(&mut problems, "a port (AIM, HAND, SELF, AMOUNT…)"),
                        Some(port) => bytes[at] = port.code,
                    }
                    at += 1;
                }
                Kind::L => {
                    let full = if token.starts_with('.') { format!("{}{}", p.scope, token) } else { token.to_string() };
                    match labels.get(&full) {
                        None => bad(&mut problems, "a label that exists"),
                        Some(&target) => bytes[at..at + 2].copy_from_slice(&(target as u16).to_le_bytes()),
                    }
                    at += 2;
                }
            }
        }
        bytes[p.addr] = p.op.code | if immediate { IMMEDIATE_BIT } else { 0 };
        // A triple port is read by IN into a triple: check the register leaves room.
        if p.op.name == "IN" {
            let port = p.tokens.get(1).and_then(|t| port_by_name(&t.to_uppercase()));
            if let (Some(port), Some(r)) = (port, N_REG.captures(&p.tokens[0]))
                && port.size == 3
                && number(r.get(1).unwrap().as_str()) > 29.0
            {
                fail(&mut problems, &p.at, format!("{} needs three registers from {}", port.name, p.tokens[0]));
            }
        }
    }

    if !problems.is_empty() {
        return Err(AsmError { problems });
    }
    Ok(Program { bytes, labels, consts, lines })
}

/// `source.split(/\r?\n/)`.
fn split_lines(source: &str) -> Vec<&str> {
    source.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

fn operand_size(kind: Kind, token: &str) -> usize {
    match kind {
        Kind::L => 2,
        Kind::I => 4,
        Kind::S if token.starts_with('#') => 4,
        _ => 1,
    }
}

/// A register's number. `IN n4:6, AIM` names a triple; the register byte is its first.
fn register(token: &str, pattern: &Regex, count: u8) -> Option<u8> {
    let m = pattern.captures(token)?;
    let r = number(m.get(1).unwrap().as_str());
    if let Some(last) = m.get(2)
        && number(last.as_str()) != r + 2.0
    {
        return None;
    }
    if r < count as f64 { Some(r as u8) } else { None }
}

fn constant(token: &str, consts: &IndexMap<String, f64>) -> Option<f64> {
    let body = token[1..].trim();
    let negative = body.starts_with('-');
    let name = if negative { &body[1..] } else { body };
    let v = if NUMBER.is_match(name) { Some(number(name)) } else { consts.get(name).copied() };
    match v {
        Some(v) if !v.is_nan() => Some(if negative { -v } else { v }),
        _ => None,
    }
}
