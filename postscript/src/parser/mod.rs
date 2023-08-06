use std::collections::HashMap;
use super::lexer::{Token::{self, *}, State};

pub struct Operation {
    op: String,
    tokens: Vec<Token>,
}

pub fn parse(mut state: State) -> Vec<Operation> {
    let mut tokens = Vec::new();

    let mut ret = Vec::new();

    while let Some(x) = state.next() {
        use std::mem::take;
        if let Operator(op) = x {
            ret.push(Operation {
                op,
                tokens: take(&mut tokens),
            });
        } else {
            tokens.push(x);
        }
    }
    ret
}

pub fn get_texts(state: State) -> Vec<Vec<Token>> {
    let operations = parse(state);
    operations.into_iter().filter(|x| x.op == "TJ").map(|x| x.tokens).collect()
}


pub fn parse_slice(mut slice: impl Iterator<Item=Token>, babel: &HashMap<u16, char>) -> String {
    let c = slice.next().unwrap();
    if c != ListStart {
        panic!()
    }

    let mut ret = String::new();

    while let Some(x) = slice.next() {
        if let StringLiteral(bytes) = x {
            let mut i = 0;
            loop {
                let x = bytes[i] as u16 * 256 + bytes[i + 1] as u16;
                ret.push(*babel.get(&x).unwrap());
                i += 2;
                if i >= bytes.len() {
                    break
                }
            }
        }
    }

    ret
}

pub fn collect(state: State, babel: &HashMap<u16, char>) -> Vec<String> {
    let segments = get_texts(state);
    let mut ret = Vec::new();
    for s in segments {
        let text = parse_slice(s.into_iter(), babel);
        ret.push(text);
    }
    ret
}
