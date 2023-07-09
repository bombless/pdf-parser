
use std::collections::VecDeque;
use std::ops::Deref;

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

impl PartialEq<str> for Token {
    fn eq(&self, other: &str) -> bool {
        if let Token::Key(s) | Token::StringLiteral(s) = self {
            s == other
        } else {
            false
        }
    }
}

impl <T> PartialEq<T> for Token where f64: From<T>, T: Clone {
    fn eq(&self, other: &T) -> bool {
        if let Token::Number(n) = self {
            n == &f64::from(other.clone())
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct State {
    store: Vec<u8>,
    index: usize,
    comments: Vec<(usize, Vec<u8>)>,
    usize_stack: VecDeque<usize>,
    tokens_waiting: VecDeque<Token>,
}

pub fn parse(src: &[u8]) -> State {
    State { store: src.into(), ..State::default() }
}

impl State {
    pub fn get_next_token(&mut self) -> Option<Token> {
        use std::mem::take;

        enum Ctx {
            Comment(usize, Vec<u8>),
            Key(usize, String),
            String(usize, String),
            None,
        }

        impl Default for Ctx {
            fn default() -> Self {
                Ctx::None
            }
        }

        if self.index >= self.store.len() {
            return self.pop_stacks();
        }

        let mut prev_ctx = Ctx::None;
        let mut curr_ctx = Ctx::None;
        
        
        let mut proc =
                |
                curr :&[_],
                token: &mut Option<_>,
                comments : &mut Vec<_>,
                index,
                tokens_waiting: &mut VecDeque<_>,
                usize_stack: &mut VecDeque<_>| {


            let byte = curr[0];

            match &mut curr_ctx {
                &mut Ctx::Comment(_, ref mut comment_content) if byte != b'\n' => {
                    comment_content.push(byte);
                    return 1;
                }
                ctx @ &mut Ctx::Comment(..) => prev_ctx = take(ctx),
                &mut Ctx::String(_, ref mut string_content) if byte != b')' => {
                    string_content.push(byte as char);
                    return 1;
                }
                ctx @ &mut Ctx::String(..) => prev_ctx = take(ctx),
                &mut Ctx::Key(_, ref mut key_content) if !byte.is_ascii_whitespace() && curr.len() > 1 => {
                    if curr.len() == 2 {
                        key_content.push(curr[0] as char);
                        key_content.push(curr[1] as char);
                        return 1;
                    }
                    key_content.push(byte as char);
                    return 1;
                }
                ctx @ &mut Ctx::Key(..) => prev_ctx = take(ctx),
                Ctx::None => {}
            }

            match take(&mut prev_ctx) {
                Ctx::Comment(base, comment_content) => {
                    comments.push((base, comment_content));
                    return 1;
                }
                Ctx::String(_, string_content) => {
                    token.replace(Token::StringLiteral(string_content));
                    return 1;
                }
                Ctx::Key(_, key_content) => {
                    token.replace(Token::Key(key_content));
                    return 1;
                }
                Ctx::None => {}
            }

            if !byte.is_ascii() { panic!("non-ascii, index {index}"); }
            let c = curr[0] as char;
            
            if c == '%' {
                curr_ctx = Ctx::Comment(index, vec![]);
                return 1;
            }

            if c == '(' {
                curr_ctx = Ctx::String(index, String::new());
                return 1;
            }

            if c == '/' {
                curr_ctx = Ctx::Key(index, String::new());
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

            if c == '-' {
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
                        usize_stack.push_back(n);
                        if usize_stack.len() > 2 {
                            tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                        }
                        return i;
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
                    tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
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

            if c == ']' {
                token.replace(Token::ListEnd);
                return 1;
            }

            if curr.starts_with(b">>") {
                token.replace(Token::DictEnd);
                return 2;
            }

            if curr.starts_with(b"\nendobj\n") {
                token.replace(Token::ObjectEnd);
                return "\nendobj\n".len();
            }
            
            if c.is_whitespace() {
                return 1;
            }

            return 0;
        };

        while self.index < self.store.len() {

            let mut token = None;
            let curr = &self.store[..][self.index..];
            let step = proc(curr, &mut token, &mut self.comments, self.index, &mut self.tokens_waiting, &mut self.usize_stack);
            self.index += step;
            if token.is_some() {
                return token;
            }
            // let item = self.pop_stacks();
            // if item.is_some() {
            //     return item;
            // }
            if step == 0 {
                return None;
            }
        }
        self.pop_stacks()

    }
    fn pop_stacks(&mut self) -> Option<Token> {
        if let Some(x) = self.tokens_waiting.pop_front() {
            return Some(x);
        }
        if let Some(x) = self.usize_stack.pop_front() {
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
    let len = src.iter().position(|x| x != &b'.' && x != &b'-' && !x.is_ascii_digit()).unwrap_or(src.len());
    src[..len].iter().map(|&x| x as char).collect::<String>().parse().ok().map(|x| (len, x))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_number() {
        assert_eq!(parse(b"1").get_next_token().unwrap(), Token::Number(1.));
        assert_eq!(parse(b"-1").get_next_token().unwrap(), Token::Number(-1.));
        assert_eq!(parse(b"1.5").get_next_token().unwrap(), Token::Number(1.5));
        assert_eq!(parse(b"1.5 2 3 4").get_next_token().unwrap(), Token::Number(1.5));
        assert_eq!(parse(b"42").get_next_token().unwrap(), Token::Number(42.));
        assert_eq!(parse(b"6 6").get_next_token().unwrap(), Token::Number(6.));
    }
    #[test]
    fn test_multiple_tokens() {
        let token = {
            let mut parser = parse(b"-1 2");
            parser.get_next_token();
            parser.get_next_token()
        };
        assert_eq!(token, Some(Token::Number(2.)));
        let token = {
            let mut parser = parse(b"1");
            parser.get_next_token();
            parser.get_next_token()
        };
        assert_eq!(token, None);
        let token = {
            let mut parser = parse(b"-1 2 3");
            parser.get_next_token();
            parser.get_next_token();
            parser.get_next_token()
        };
        assert_eq!(token, Some(Token::Number(3.)));

        let mut parser = parse(b"1 2 3 4 5");
        assert_eq!(parser.get_next_token().unwrap(), 1);
        assert_eq!(parser.get_next_token().unwrap(), 2);
        assert_eq!(parser.get_next_token().unwrap(), 3);
        assert_eq!(parser.get_next_token().unwrap(), 4);
        assert_eq!(parser.get_next_token().unwrap(), 5);
        assert_eq!(parser.get_next_token(), None);
    }
    #[test]
    fn test_object() {
        assert_eq!(parse(b"1 2 obj\n").get_next_token().unwrap(), Token::ObjectStart((1, 2)));
        assert_eq!(parse(b"\nendobj\n").get_next_token().unwrap(), Token::ObjectEnd);
    }
    #[test]
    fn test_key_or_string() {
        assert_eq!(&parse(b"/abc").get_next_token().unwrap(), "abc");
        assert_eq!(&parse(b"(I love you)").get_next_token().unwrap(), "I love you");
    }

}