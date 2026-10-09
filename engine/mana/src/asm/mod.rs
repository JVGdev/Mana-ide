//! The assembly: the instruction set, the assembler (`.masm` → bytes) and the disassembler (bytes → instructions).

pub mod assembler;
pub mod disassembler;
pub mod docs;
pub mod isa;

pub use assembler::{AsmError, Program, SourceLine, assemble};
pub use disassembler::{DecodeError, Instr, decode, decode_all, format, listing};
