
use std::collections::VecDeque;
use std::fmt;

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
    XRef,
    Eof,
    Id(u128),
    Null,
    Bool(bool),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;
        match self {
            Bool(b) => write!(f, "{b}"),
            Null => write!(f, "null"),
            StringLiteral(s) => write!(f, "{s:?}"),
            Key(s) => write!(f, "/{s}"),
            DictStart => write!(f, "DictStart"),
            DictEnd => write!(f, "DictEnd"),
            ListStart => write!(f, "ListStart"),
            ListEnd => write!(f, "ListEnd"),
            StreamStart => write!(f, "StreamStart"),
            StreamEnd => write!(f, "StreamEnd"),
            ObjectStart(id) => write!(f, "ObjectStart{id:?}"),
            ObjectEnd => write!(f, "ObjectEnd"),
            Ref(id) => write!(f, "Ref{id:?}"),
            Number(n) => write!(f, "Number({n})"),
            XRef => write!(f, "xref"),
            Eof => write!(f, "EOF"),
            Id(x) => write!(f, "<{x}>")
        }
    }
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
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn len(&self) -> usize {
        self.store.len()
    }
    pub fn is(&mut self, t: Token) -> bool {
        if !self.tokens_waiting.is_empty() {
            return Some(&t) == self.tokens_waiting.get(0);
        }
        let next = self.next();
        match next {
            None => false,
            Some(next) if next != t => {
                self.tokens_waiting.push_front(next);
                false
            }
            _ => {
                self.tokens_waiting.push_front(t);
                true
            }
        }
    }
    pub fn swallow(&mut self, t: Token) {
        self.tokens_waiting.push_front(t);
    }
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

        if self.usize_stack.is_empty() && !self.tokens_waiting.is_empty() {
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
                    if byte == b'\\' {
                        if curr.len() > 1 && (curr[1] == b'(' || curr[1] == b')') {
                            string_content.push(curr[1] as char);
                            return 2;
                        }
                        return 0;
                    }
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

                if n == 0 && curr[1] != b'.' {
                    return if curr[1].is_ascii_digit() {
                        0
                    } else {
                        usize_stack.push_back(n);
                        1
                    };
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
                if usize_stack.len() >= 2 {
                    while usize_stack.len() > 2 {
                        tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                    }
                    token.replace(Token::ObjectStart((usize_stack[0], usize_stack[1])));
                    usize_stack.clear();
                    return "obj\n".len();
                }
                return 0;
            }

            if curr.starts_with(b"R") {
                if usize_stack.len() >= 2 {
                    while usize_stack.len() > 2 {
                        tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                    }
                    token.replace(Token::Ref((usize_stack[0], usize_stack[1])));
                    usize_stack.clear();
                    return "R".len();
                }
                return 0;
            }

            if curr.starts_with(b"endstream\n") {
                assert_eq!(0, usize_stack.len());
                assert_eq!(0, tokens_waiting.len());
                token.replace(Token::StreamEnd);
                return b"endstream\n".len();
            }

            if c == ']' {
                while !usize_stack.is_empty() {
                    tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                }
                tokens_waiting.push_back(Token::ListEnd);
                return 1;
            }

            if curr.starts_with(b">>") {
                while !usize_stack.is_empty() {
                    tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                }
                token.replace(Token::DictEnd);
                return 2;
            }

            if curr.starts_with(b"endobj\n") {
                token.replace(Token::ObjectEnd);
                return "endobj\n".len();
            }

            if curr.starts_with(b"xref\n") {
                token.replace(Token::XRef);
                return b"xref\n".len();
            }
            
            if c.is_whitespace() {
                return 1;
            }

            if byte == b'<' {
                let id = String::from_utf8(curr[1..33].into()).unwrap();
                token.replace(Token::Id(u128::from_str_radix(&id, 16).unwrap()));
                return 34;
            }

            if curr.starts_with(b"null") && !curr[4].is_ascii_alphanumeric() {
                token.replace(Token::Null);
                return 4;
            }

            if curr.starts_with(b"true") && !curr[4].is_ascii_alphanumeric() {
                token.replace(Token::Bool(true));
                return 4;
            }

            if curr.starts_with(b"false") && !curr[5].is_ascii_alphanumeric() {
                token.replace(Token::Bool(false));
                return 5;
            }

            return 0;
        };

        while self.index < self.store.len() {

            let mut token = None;
            let curr = &self.store[..][self.index..];
            let step = proc(curr, &mut token, &mut self.comments, self.index, &mut self.tokens_waiting, &mut self.usize_stack);
            self.index += step;
            if let Some(token) = token {
                return match self.pop_stacks() {
                    None => Some(token),
                    Some(x) => {
                        self.tokens_waiting.push_back(token);
                        Some(x)
                    }
                };
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
    pub fn get_fixed_length_stream(&mut self, size: usize, buf: &mut Vec<u8>) -> usize {
        if self.store.len() < size + self.index {
            return 0;
        }
        buf.extend(&self.store[self.index..self.index+size]);
        self.index += size;
        size
    }
    pub fn get_flate_stream(&mut self, size: usize, buf: &mut Vec<u8>) -> usize {
        if self.store.len() < size + self.index {
            return 0;
        }

        fn decode(data: &[u8], buf: &mut Vec<u8>) -> usize {
            use std::io::Read;
            use flate2::bufread::ZlibDecoder;
        
            let mut decoder = ZlibDecoder::new(data);
            
            decoder.read_to_end(buf).unwrap()
        }

        decode(&self.store[..][self.index .. self.index + size], buf);

        self.index += size;

        size        
    }
    pub fn get_ascii_line(&mut self) -> Option<String> {
        let mut i = 0;
        let mut ret = String::new();
        while self.index + i < self.store.len() && self.store[self.index + i].is_ascii() {
            let byte = self.store[self.index + i];
            if byte == b'\n' {
                break;
            }
            ret.push(byte as _);
            i += 1;
        }

        if self.index + i >= self.store.len() {
            return if ret.is_empty() {
                None
            } else {
                self.index += i + 1;
                Some(ret)
            };
        }

        if !self.store[self.index + i].is_ascii() {
            return None;
        }

        self.index += i + 1;

        Some(ret)

    }
}

fn parse_number(src: &[u8]) -> Option<(usize, f64)> {
    let len = src.iter().position(|x| x != &b'.' && x != &b'-' && !x.is_ascii_digit()).unwrap_or(src.len());
    src[..len].iter().map(|&x| x as char).collect::<String>().parse().ok().map(|x| (len, x))
}

impl Iterator for State {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        self.get_next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_number() {
        assert_eq!(parse(b"0.9505").get_next_token().unwrap(), Token::Number(0.9505));
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
    #[test]
    fn test_dict_value_0() {
        use Token::*;
        let expr = [DictStart, Key("abc".into()), Number(0.), DictEnd];
        assert_eq!(parse(b"<</abc 0>>").collect::<Vec<_>>(), expr);
    }

    #[test]
    fn test_mixed() {
        use Token::*;
        let expr = [
            DictStart,
            Key("a".into()),
            ListStart,
            Ref((4, 0)),
            ListEnd,
            Key("b".into()),
            Ref((6, 0)),
            DictEnd,
        ];
        assert_eq!(parse(b"<< /a [4 0 R] /b 6 0 R >>").collect::<Vec<_>>(), expr);

        let expr = [
            ListStart,
            Number(0.9505),
            Number(1.),
            Number(1.0888),
            ListEnd,
        ];
        assert_eq!(parse(b"[0.9505 1 1.0888]").collect::<Vec<_>>(), expr)
    }

}