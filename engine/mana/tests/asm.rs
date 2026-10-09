//! The assembler and disassembler (from test/asm.test.ts).

use std::collections::HashMap;

use mana::asm::docs::{op_doc, port_doc};
use mana::asm::isa::{OPS, PORTS};
use mana::asm::{Program, assemble, decode_all, format, listing};
use mana::load::{resolver, spell};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

fn asm(src: &str) -> Program {
    assemble(src, "test.masm", &resolver(vec![])).unwrap_or_else(|e| panic!("{e}"))
}

mod encoding {
    use super::*;

    #[test]
    fn encodes_the_examples_in_the_spec() {
        assert_eq!(hex(&asm("GATH m0, n17").bytes), "30 80 11");
        assert_eq!(hex(&asm("FILT m1, m0, #3").bytes), "B2 81 80 00 00 40 40");
        assert_eq!(hex(&asm("EMIT m1, n6, n4, n13:15").bytes), "52 81 06 04 0D");
    }

    #[test]
    fn sets_the_top_bit_only_when_the_last_operand_is_an_immediate() {
        assert_eq!(asm("ADD n1, n2").bytes[0], 0x12);
        assert_eq!(asm("ADD n1, #2").bytes[0], 0x92);
    }

    #[test]
    fn reads_constants_from_libraries_it_uses() {
        let p = asm(".use Elements\n FILT m1, m0, #EARTH");
        assert_eq!(hex(&p.bytes), "B2 81 80 00 00 40 40");
        assert_eq!(p.consts.get("FIRE"), Some(&0.0));
    }

    #[test]
    fn scopes_local_labels_to_the_global_label_before_them() {
        let p = asm("a:  JMP .x\n.x: NOP\nb:  JMP .x\n.x: HALT");
        assert_eq!(p.labels.get("a.x"), Some(&3));
        assert_eq!(p.labels.get("b.x"), Some(&7));
        assert_eq!(u16::from_le_bytes([p.bytes[5], p.bytes[6]]), 7);
    }

    #[test]
    fn takes_in_each_library_once_after_the_spell() {
        let p = asm(".use Basics, Basics\nspell: HALT");
        assert_eq!(p.labels.get("spell"), Some(&0));
        assert_eq!(p.labels.get("toward"), Some(&1));
    }
}

mod decoding {
    use super::*;

    #[test]
    fn reads_back_what_it_wrote() {
        let src = "
start:  IN    n16:18, AIM
        LDI   n0, #2.5
        PROB  n0, n16:18, #3
        LOCK  n20, INPUT
        PUTW  #4, n4
        LD    n1, [n2]
        JMP   start";
        let p = asm(src);
        let labels = HashMap::from([(0, "start".to_string())]);
        let lines: Vec<String> = decode_all(&p.bytes).unwrap().iter().map(|i| format(i, Some(&labels))).collect();
        assert_eq!(
            lines,
            [
                "IN    n16:18, AIM",
                "LDI   n0, #2.5",
                "PROB  n0, n16:18, #3",
                "LOCK  n20, INPUT",
                "PUTW  #4, n4",
                "LD    n1, [n2]",
                "JMP   start",
            ]
        );
    }

    #[test]
    fn lists_the_four_spells_without_a_hitch() {
        for name in ["StoneWall", "Fireball", "Gust", "WaterShield"] {
            let p = spell(name);
            assert!(listing(&p.bytes, Some(&p.labels)).unwrap().contains(&format!("{name}:")));
        }
    }
}

mod mistakes {
    use super::*;

    fn problems(src: &str) -> Vec<String> {
        match assemble(src, "test.masm", &resolver(vec![])) {
            Ok(_) => vec![],
            Err(e) => e.problems,
        }
    }

    #[test]
    fn names_the_line_and_what_was_wrong() {
        assert_eq!(problems("NOP\nFLY n1"), ["test.masm:2: unknown instruction FLY"]);
        assert!(problems("GATH n0, n1")[0].contains("operand 1: a mana register"));
        assert!(problems("EMIT m1, n6, n4, n13:14")[0].contains("three number registers"));
        assert!(problems("JMP nowhere")[0].contains("a label that exists"));
        assert!(problems("FILT m1, m0, #EARTH")[0].contains("a known #NAME"));
        assert!(problems(".use Nothing")[0].contains("no library called Nothing"));
        assert!(problems("ADD n1")[0].contains("takes 2 operand"));
    }
}

mod docs {
    use super::*;

    #[test]
    fn say_what_every_instruction_and_port_does() {
        for op in &OPS {
            assert!(op_doc(op.name).is_some_and(|d| d.syntax.starts_with(op.name)), "{}", op.name);
        }
        for p in &PORTS {
            assert!(port_doc(p.name).is_some_and(|d| !d.is_empty()), "{}", p.name);
        }
    }
}
