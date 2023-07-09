#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    StringLiteral(String),
    Key(String),
    DictStart,
    DictEnd,
    ListStart,
    ListEnd,
    StreamStart,
    StreamEnd,
    ObjectStart(usize, usize),
    ObjectEnd,
    Ref(usize, usize),
    Number,
}

pub struct State {
    store: Vec<u8>,
    index: usize,
    comments: Vec<(usize, Vec<u8>)>,
}

pub fn parse(src: &[u8]) -> State {
    State { store: src.into(), index: 0, comments: vec![] }
}

impl State {
    pub fn get_next_token(&mut self) -> Option<Token> {
        use std::mem::take;

        if self.index >= self.store.len() {
            return None;
        }

        let mut comment : Option<(usize, Vec<u8>)> = None;

        let mut token = None;
        
        let mut proc =
                |curr :&[u8], token: &mut Option<Token>, index| {
            let byte = curr[0];

            if let &mut Some((base, ref mut comment)) = &mut comment {
                if byte != b'\n' {
                    comment.push(byte);
                }
                else {
                    self.comments.push((base, take(comment)));
                }
                return 1;
            }

            if !byte.is_ascii() { panic!("non-ascii, index {index}"); }
            let c = curr[0] as char;
            
            if c.is_whitespace() { return 1; }
            
            if c == '%' {
                comment = Some((index, vec![]));
                return 1;
            }

            if curr.starts_with(b"<<") {
                *token = Some(Token::DictStart);
                return 2;
            }

            if c == '[' {
                *token = Some(Token::ListStart);
                return 1;
            }

            if curr.starts_with(b"\nstream\n") {
                *token = Some(Token::StreamStart);
                return "\nstream\n".len();
            }

            return 0;
        };

        while self.index < self.store.len() {
            let curr = &self.store[..][self.index..];
            let step = proc(curr, &mut token, self.index);
            if step == 0 { return None; }
            self.index += step;
            if token.is_some() {
                return token;
            }
        }

        None

    }
    pub fn get_fixed_length_stream(&mut self, size: usize, buf: &mut Vec<u8>) -> bool {
        if self.store.len() < size + self.index {
            return false;
        }
        buf.extend(&self.store[self.index..size]);
        true
    }
    pub fn get_flate_stream(&mut self, buf: &mut Vec<u8>) -> bool {
        const END: &'static [u8] = b"\nendstream\n;";
        const LEN: usize = END.len();

        if let Some(pos) = self.store[..][self.index..].windows(LEN).position(|x| x.starts_with(END)) {
            buf.extend(&self.store[..pos]);
            true
        } else {
            false
        }        
    }
}
