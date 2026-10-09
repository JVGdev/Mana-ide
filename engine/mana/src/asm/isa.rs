//! The instruction set: one table that the assembler, the disassembler and the machine all read.
//!
//! Operand kinds:
//!   n  a number register (n0–n31)          one byte, top bit 0
//!   m  a mana register (m0–m7)             one byte, top bit 1
//!   s  a number register or an immediate   a register byte, or four bytes (float32) when it's an immediate;
//!                                          only ever the last operand, and an immediate sets the opcode's top bit
//!   t  a triple: the first of three number registers
//!   a  an address held in a register, written [n]
//!   L  a label                             two bytes, the address
//!   P  a port                              one byte
//!   K  a small constant (#k, SHAPE…)       one byte
//!   I  an immediate                        four bytes (float32)

/// What kind of operand an instruction takes (see above).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    N,
    M,
    S,
    T,
    A,
    L,
    P,
    K,
    I,
}

/// Every instruction, by name: what the machine dispatches on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mn {
    Nop,
    Halt,
    Fail,
    Tick,
    Ret,
    Jmp,
    Jeq,
    Jne,
    Jlt,
    Jle,
    Jgt,
    Jge,
    Call,
    Ldi,
    Mov,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Abs,
    Sqrt,
    Floor,
    Round,
    Sin,
    Cos,
    Tan,
    Atan,
    Atn2,
    Min,
    Max,
    Cmp,
    Push,
    Pop,
    Ld,
    St,
    In,
    Gath,
    Circ,
    Filt,
    Splt,
    Join,
    Meas,
    Part,
    Vent,
    Prob,
    Airm,
    Send,
    Wpos,
    Wvel,
    Weav,
    Turn,
    Emit,
    Wset,
    Wget,
    Ordr,
    Mani,
    Lock,
    Rels,
    Pcnt,
    Ppos,
    Pvel,
    Shov,
    Ingr,
    Kick,
    Tuch,
    Getw,
    Putw,
    Diss,
    Cnds,
    Dens,
    Grad,
    Nvel,
}

#[derive(Debug)]
pub struct Op {
    pub code: u8,
    pub name: &'static str,
    pub mn: Mn,
    pub operands: &'static [Kind],
    /// Beats of thought it takes.
    pub beats: u32,
    /// Only inside a particle's order.
    pub order: bool,
}

// What thinking costs, in beats. Adding is quick, dividing is slow, and a sine is slower still: the costs a mind pays are
// the costs a real processor pays, so the same tricks make a spell faster (SPEC D18).
const MIND: u32 = 1;
const SLOW: u32 = 4;
const TRIG: u32 = 8;
const BODY: u32 = 4;

use Kind::*;

pub static OPS: [Op; 74] = [
    Op { code: 0x00, name: "NOP", mn: Mn::Nop, operands: &[], beats: MIND, order: false },
    Op { code: 0x01, name: "HALT", mn: Mn::Halt, operands: &[], beats: MIND, order: false },
    Op { code: 0x02, name: "FAIL", mn: Mn::Fail, operands: &[K], beats: MIND, order: false },
    Op { code: 0x03, name: "TICK", mn: Mn::Tick, operands: &[], beats: MIND, order: false },
    Op { code: 0x07, name: "RET", mn: Mn::Ret, operands: &[], beats: MIND, order: false },
    Op { code: 0x08, name: "JMP", mn: Mn::Jmp, operands: &[L], beats: MIND, order: false },
    Op { code: 0x09, name: "JEQ", mn: Mn::Jeq, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0a, name: "JNE", mn: Mn::Jne, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0b, name: "JLT", mn: Mn::Jlt, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0c, name: "JLE", mn: Mn::Jle, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0d, name: "JGT", mn: Mn::Jgt, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0e, name: "JGE", mn: Mn::Jge, operands: &[L], beats: MIND, order: false },
    Op { code: 0x0f, name: "CALL", mn: Mn::Call, operands: &[L], beats: MIND, order: false },
    Op { code: 0x10, name: "LDI", mn: Mn::Ldi, operands: &[N, I], beats: MIND, order: false },
    Op { code: 0x11, name: "MOV", mn: Mn::Mov, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x12, name: "ADD", mn: Mn::Add, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x13, name: "SUB", mn: Mn::Sub, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x14, name: "MUL", mn: Mn::Mul, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x15, name: "DIV", mn: Mn::Div, operands: &[N, S], beats: SLOW, order: false },
    Op { code: 0x16, name: "MOD", mn: Mn::Mod, operands: &[N, S], beats: SLOW, order: false },
    Op { code: 0x17, name: "NEG", mn: Mn::Neg, operands: &[N], beats: MIND, order: false },
    Op { code: 0x18, name: "ABS", mn: Mn::Abs, operands: &[N], beats: MIND, order: false },
    Op { code: 0x19, name: "SQRT", mn: Mn::Sqrt, operands: &[N], beats: SLOW, order: false },
    Op { code: 0x1a, name: "FLOOR", mn: Mn::Floor, operands: &[N], beats: MIND, order: false },
    Op { code: 0x1b, name: "ROUND", mn: Mn::Round, operands: &[N], beats: MIND, order: false },
    Op { code: 0x1c, name: "SIN", mn: Mn::Sin, operands: &[N], beats: TRIG, order: false },
    Op { code: 0x1d, name: "COS", mn: Mn::Cos, operands: &[N], beats: TRIG, order: false },
    Op { code: 0x1e, name: "TAN", mn: Mn::Tan, operands: &[N], beats: TRIG, order: false },
    Op { code: 0x1f, name: "ATAN", mn: Mn::Atan, operands: &[N], beats: TRIG, order: false },
    Op { code: 0x20, name: "ATN2", mn: Mn::Atn2, operands: &[N, S], beats: TRIG, order: false },
    Op { code: 0x21, name: "MIN", mn: Mn::Min, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x22, name: "MAX", mn: Mn::Max, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x23, name: "CMP", mn: Mn::Cmp, operands: &[N, S], beats: MIND, order: false },
    Op { code: 0x24, name: "PUSH", mn: Mn::Push, operands: &[N], beats: MIND, order: false },
    Op { code: 0x25, name: "POP", mn: Mn::Pop, operands: &[N], beats: MIND, order: false },
    Op { code: 0x26, name: "LD", mn: Mn::Ld, operands: &[N, A], beats: MIND, order: false },
    Op { code: 0x27, name: "ST", mn: Mn::St, operands: &[N, A], beats: MIND, order: false },
    Op { code: 0x28, name: "IN", mn: Mn::In, operands: &[N, P], beats: MIND, order: false },
    Op { code: 0x30, name: "GATH", mn: Mn::Gath, operands: &[M, S], beats: BODY, order: false },
    Op { code: 0x31, name: "CIRC", mn: Mn::Circ, operands: &[M], beats: BODY, order: false },
    Op { code: 0x32, name: "FILT", mn: Mn::Filt, operands: &[M, M, S], beats: BODY, order: false },
    Op { code: 0x33, name: "SPLT", mn: Mn::Splt, operands: &[M, M, S], beats: BODY, order: false },
    Op { code: 0x34, name: "JOIN", mn: Mn::Join, operands: &[M, M], beats: BODY, order: false },
    Op { code: 0x35, name: "MEAS", mn: Mn::Meas, operands: &[N, M], beats: BODY, order: false },
    Op { code: 0x36, name: "PART", mn: Mn::Part, operands: &[N, M, S], beats: BODY, order: false },
    Op { code: 0x37, name: "VENT", mn: Mn::Vent, operands: &[M], beats: BODY, order: false },
    Op { code: 0x40, name: "PROB", mn: Mn::Prob, operands: &[N, T, S], beats: BODY, order: false },
    Op { code: 0x41, name: "AIRM", mn: Mn::Airm, operands: &[N, T, S], beats: BODY, order: false },
    Op { code: 0x42, name: "SEND", mn: Mn::Send, operands: &[M, N, T, T], beats: BODY, order: false },
    Op { code: 0x43, name: "WPOS", mn: Mn::Wpos, operands: &[T, N], beats: BODY, order: false },
    Op { code: 0x44, name: "WVEL", mn: Mn::Wvel, operands: &[T, N], beats: BODY, order: false },
    Op { code: 0x50, name: "WEAV", mn: Mn::Weav, operands: &[N, T], beats: BODY, order: false },
    Op { code: 0x51, name: "TURN", mn: Mn::Turn, operands: &[N, T], beats: BODY, order: false },
    Op { code: 0x52, name: "EMIT", mn: Mn::Emit, operands: &[M, N, N, T], beats: BODY, order: false },
    Op { code: 0x53, name: "WSET", mn: Mn::Wset, operands: &[N, K, S], beats: BODY, order: false },
    Op { code: 0x54, name: "WGET", mn: Mn::Wget, operands: &[N, N, K], beats: BODY, order: false },
    Op { code: 0x55, name: "ORDR", mn: Mn::Ordr, operands: &[N, L], beats: BODY, order: false },
    Op { code: 0x56, name: "MANI", mn: Mn::Mani, operands: &[N], beats: BODY, order: false },
    Op { code: 0x57, name: "LOCK", mn: Mn::Lock, operands: &[N, K], beats: BODY, order: false },
    Op { code: 0x58, name: "RELS", mn: Mn::Rels, operands: &[N], beats: BODY, order: false },
    Op { code: 0x59, name: "PCNT", mn: Mn::Pcnt, operands: &[N, N], beats: BODY, order: false },
    Op { code: 0x5a, name: "PPOS", mn: Mn::Ppos, operands: &[T, N, S], beats: BODY, order: false },
    Op { code: 0x5b, name: "PVEL", mn: Mn::Pvel, operands: &[T, N, S], beats: BODY, order: false },
    Op { code: 0x5c, name: "SHOV", mn: Mn::Shov, operands: &[M, N, N, T], beats: BODY, order: false },
    Op { code: 0x5d, name: "INGR", mn: Mn::Ingr, operands: &[N, S], beats: BODY, order: false },
    Op { code: 0x60, name: "KICK", mn: Mn::Kick, operands: &[T], beats: BODY, order: true },
    Op { code: 0x61, name: "TUCH", mn: Mn::Tuch, operands: &[N], beats: MIND, order: true },
    Op { code: 0x62, name: "GETW", mn: Mn::Getw, operands: &[N, K], beats: MIND, order: true },
    Op { code: 0x63, name: "PUTW", mn: Mn::Putw, operands: &[K, S], beats: MIND, order: true },
    Op { code: 0x64, name: "DISS", mn: Mn::Diss, operands: &[], beats: MIND, order: true },
    Op { code: 0x65, name: "CNDS", mn: Mn::Cnds, operands: &[S], beats: MIND, order: true },
    Op { code: 0x66, name: "DENS", mn: Mn::Dens, operands: &[N], beats: BODY, order: true },
    Op { code: 0x67, name: "GRAD", mn: Mn::Grad, operands: &[T], beats: BODY, order: true },
    Op { code: 0x68, name: "NVEL", mn: Mn::Nvel, operands: &[T], beats: BODY, order: true },
];

pub const IMMEDIATE_BIT: u8 = 0x80;
pub const MANA_BIT: u8 = 0x80;

pub fn op_by_name(name: &str) -> Option<&'static Op> {
    OPS.iter().find(|o| o.name == name)
}

pub fn op_by_code(code: u8) -> Option<&'static Op> {
    OPS.iter().find(|o| o.code == code)
}

/// A port, and how many numbers it reads.
#[derive(Debug)]
pub struct Port {
    pub name: &'static str,
    pub code: u8,
    pub size: usize,
    pub order: bool,
}

pub static PORTS: [Port; 12] = [
    Port { name: "AIM", code: 0x00, size: 3, order: false },
    Port { name: "HAND", code: 0x01, size: 3, order: false },
    Port { name: "SELF", code: 0x02, size: 3, order: false },
    Port { name: "AMOUNT", code: 0x03, size: 1, order: false },
    Port { name: "FORCE", code: 0x04, size: 1, order: false },
    Port { name: "MAINTAIN", code: 0x05, size: 1, order: false },
    Port { name: "CELL", code: 0x06, size: 1, order: false },
    Port { name: "DEPTH", code: 0x07, size: 1, order: false },
    Port { name: "LOAD", code: 0x08, size: 1, order: false },
    Port { name: "CAPACITY", code: 0x09, size: 1, order: false },
    Port { name: "REACH", code: 0x0c, size: 1, order: false },
    Port { name: "VEL", code: 0x0d, size: 3, order: true },
];

pub fn port_by_name(name: &str) -> Option<&'static Port> {
    PORTS.iter().find(|p| p.name == name)
}

pub fn port_by_code(code: u8) -> Option<&'static Port> {
    PORTS.iter().find(|p| p.code == code)
}

/// Names a K operand can be written as. A shape isn't locked any more: it's held, by pushing (SPEC §11). #0 is left where
/// SHAPE was, and locks nothing.
pub fn lock(name: &str) -> Option<u8> {
    match name {
        "INPUT" => Some(1),
        "ORDER" => Some(2),
        _ => None,
    }
}

pub const LOCK_NAMES: [&str; 3] = ["", "INPUT", "ORDER"];
