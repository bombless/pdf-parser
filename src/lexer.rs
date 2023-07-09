
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StringLiteral(String),
    Key(String),
    DictStart,
    DictEnd,
    ListStart,
    ListEnd,
    StreamStart,
    StreamEnd,
    ObjectStart((usize, usize)),
    ObjectEnd,
    Ref((usize, usize)),
    Number(f64),
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

        let mut usize_stack = VecDeque::new();

        let mut tokens_waiting = VecDeque::new();

        let mut comment : Option<(usize, Vec<u8>)> = None;
        
        let mut proc =
                |
                curr :&[_],
                token: &mut Option<_>,
                index,
                tokens_waiting: &mut VecDeque<_>,
                usize_stack: &mut VecDeque<_>| {


            let byte = curr[0];

            let mut comment_end = false;

            if let &mut Some((base, ref mut comment_content)) = &mut comment {
                if byte != b'\n' {
                    comment_content.push(byte);
                    return 1;
                }
                else {
                    self.comments.push((base, take(comment_content)));
                    comment_end = true;
                }
            }
            if comment_end {
                comment = None;
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
                token.replace(Token::DictStart);
                return 2;
            }

            if c == '[' {
                token.replace(Token::ListStart);
                return 1;
            }

            if curr.starts_with(b"\nstream\n") {
                token.replace(Token::StreamStart);
                return "\nstream\n".len();
            }

            if c.is_digit(10) {
                let n = (byte - b'0') as usize;

                if n == 0  {
                    if curr[1].is_ascii_digit() {
                        return 0;
                    } else {
                        usize_stack.push_back(n);
                        return 1;
                    }
                }
                
                let mut n = n;

                for i in 1 .. curr.len() {
                    if curr[i].is_ascii_digit() {
                        n = n * 10 + (curr[i] - b'0') as usize;
                        continue;
                    }
                    if curr[i] != b'.' {
                        break;
                    }
                    let (len, n) = if let Some((len, n)) = parse_number(curr) {
                        (len, n)
                    } else {
                        return 0;
                    };
                    if !usize_stack.is_empty() {
                        let numbers = usize_stack
                            .drain(..)
                            .map(|x| Token::Number( x as f64));
                        tokens_waiting.extend(numbers);
                    }
                    tokens_waiting.push_back(Token::Number(n));
                    return len;
                }
                usize_stack.push_back(n);
                if usize_stack.len() > 2 {
                    token.replace(Token::Number(usize_stack.pop_front().unwrap() as _));
                }
                return curr.len();
            }

            if curr.starts_with(b"obj\n") {
                if usize_stack.len() == 2 {
                    token.replace(Token::ObjectStart((usize_stack[0], usize_stack[1])));
                    usize_stack.clear();
                    return "obj\n".len();
                }
                return 0;
            }

            if curr.starts_with(b"R") {
                if usize_stack.len() == 2 {
                    token.replace(Token::Ref((usize_stack[0], usize_stack[1])));
                    usize_stack.clear();
                    return "R\n".len();
                }
                return 0;
            }

            if usize_stack.len() >= 2 {
                token.replace(Token::Number(usize_stack.pop_front().unwrap() as _));
            }

            return 0;
        };

        while self.index < self.store.len() {

            let mut token = None;
            let curr = &self.store[..][self.index..];
            let step = proc(curr, &mut token, self.index, &mut tokens_waiting, &mut usize_stack);
            if token.is_some() {
                return token;
            }
            if let Some(x) = tokens_waiting.pop_front() {
                return Some(x);
            }
            if let Some(x) = usize_stack.pop_front() {
                return Some(Token::Number(x as _));
            }
            if step == 0 { return None; }
            self.index += step;
        }
        if let Some(x) = tokens_waiting.pop_front() {
            return Some(x);
        }
        if let Some(x) = usize_stack.pop_front() {
            return Some(Token::Number(x as _));
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

fn parse_number(src: &[u8]) -> Option<(usize, f64)> {
    let len = src.iter().position(|x| x != &b'.' && !x.is_ascii_digit()).unwrap_or(src.len());
    src[..len].iter().map(|&x| x as char).collect::<String>().parse().ok().map(|x| (len, x))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_number() {
        assert_eq!(parse(b"1").get_next_token().unwrap(), Token::Number(1.));
        assert_eq!(parse(b"1.5").get_next_token().unwrap(), Token::Number(1.5));
        assert_eq!(parse(b"1.5 2 3 4").get_next_token().unwrap(), Token::Number(1.5));
        assert_eq!(parse(b"42").get_next_token().unwrap(), Token::Number(42.));
        assert_eq!(parse(b"6 6").get_next_token().unwrap(), Token::Number(6.));
    }
    #[test]
    fn test_object() {
        assert_eq!(parse(b"1 2 obj\n").get_next_token().unwrap(), Token::ObjectStart((1, 2)));
    }

}