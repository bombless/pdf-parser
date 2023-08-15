use super::operation::TextState;
use postscript::parser::{Operation, get_one_string};
use postscript::lexer::Token::*;
use std::collections::HashMap;    

pub fn handle_text_operation(op: Operation, state: &mut TextState, babel: &HashMap<u16, char>) {
    match &*op.op {
        "Tf" => {
            let size = if let Some(&Number(n)) = op.tokens.get(1) {
                n
            } else {
                panic!("wat");
            };
            state.set_font_size(size);
        }
        "Tm" => {
            let e = if let Some(&Number(e)) = op.tokens.get(4) {
                e
            } else {
                panic!("wat");
            };
            let f = if let Some(&Number(f)) = op.tokens.get(4) {
                f
            } else {
                panic!("wat");
            };
            state.set_pos(e, f);

        }
        "Td" => {
            let x_offset = if let Some(Number(x)) = op.tokens.get(0) {
                x
            } else {
                panic!("wat");
            };
            let y_offset = if let Some(Number(y)) = op.tokens.get(0) {
                y
            } else {
                panic!("wat");
            };
            let (x, y) = state.get_pos();
            state.set_pos(x + x_offset, y + y_offset);
        }
        "TJ" => {
            for operand in op.tokens {
                if let Number(n) = &operand {
                    let size = state.get_font_size();
                    let (x, y) = state.get_pos();
                    state.set_pos(x - n / 1000. * size, y);
                }
                if let Some(s) = get_one_string(operand, babel) {
                    let size = state.get_font_size();
                    let (x, y) = state.get_pos();
                    let len = s.len();
                    state.push(s);
                    state.set_pos(x + len as f64 * size, y);
                }
            }
        }
        _ => {},
    }
}
