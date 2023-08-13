
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StringLiteral(Vec<u8>),
    BytesLiteral(Vec<u8>),
    Key(String),
    Operator(String),
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
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;
        match self {
            StringLiteral(s) | BytesLiteral(s) => write!(f, "{s:?}"),
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
            Operator(s) => write!(f, "{s}"),
        }
    }
}

impl PartialEq<str> for Token {
    fn eq(&self, other: &str) -> bool {
        match self {
            Token::Key(s) if s == other => true,
            Token::StringLiteral(s) | Token::BytesLiteral(s) if s == other.as_bytes() => true,
            _ => false
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
            String(usize, Vec<u8>),
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

            if byte == b'<' && Some(&b'<') != curr.get(1) {
                let mut data = Vec::new();
                let mut factor = 0;
                for i in 1 .. curr.len() {
                    let b = curr[i];
                    if b == b'>' {
                        if factor == 0 {
                            token.replace(Token::BytesLiteral(data));
                            return i + 1;
                        }
                        return 0;
                    }
                    if b >= b'0' && b <= b'9' {
                        factor = factor * 16 + (b - b'0');
                    }
                    else if b >= b'a' && b <= b'f' {
                        factor = factor * 16 + (b - b'a' + 10);
                    }
                    else if b >= b'A' && b <= b'F' {
                        factor = factor * 16 + (b - b'A' + 10);
                    }
                    else {
                        return 0;
                    }
                    if i % 2 == 0 {
                        data.push(factor);
                        factor = 0;
                    }
                }
            }

            match &mut curr_ctx {
                &mut Ctx::Comment(_, ref mut comment_content) if byte != b'\n' => {
                    comment_content.push(byte);
                    return 1;
                }
                ctx @ &mut Ctx::Comment(..) => prev_ctx = take(ctx),
                &mut Ctx::String(_, ref mut string_content) if byte != b')' => {
                    string_content.push(byte);
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
                curr_ctx = Ctx::String(index, Vec::new());
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

            if c.is_alphabetic() {
                let slice = curr.iter().cloned().take_while(|x| x.is_ascii_alphabetic());

                let operator = String::from_utf8(slice.collect()).unwrap();
                let len = operator.len();
                while !usize_stack.is_empty() {
                    tokens_waiting.push_back(Token::Number(usize_stack.pop_front().unwrap() as _));
                }
                token.replace(Token::Operator(operator));
                return len;
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
    use Token::*;

    macro_rules! helper  {
        ($($e:ident $arr:tt)+) => {
            {
                let mut ret = Vec::new();
                $(ret.extend(helper_helper!($e $arr));)+
                ret
            }
        };
    }

    macro_rules!  helper_helper {
        ($e:ident $) => {
            [$e]
        };
        ($e:ident $arr:tt) => {
            $arr.into_iter().map(|x| $e(x.into()))
        }
    }

    #[test]
    fn test_bytestring() {
        let mut state = parse(b"<200d0a>");
        assert_eq!(&state.next().unwrap(), " \r\n");
    }

    #[test]
    fn test() {
        let state = parse(br#"
        1 0 0 -1 0 841.89105 cm
        /srgb cs
        0 0 0 scn
        /F0 11 Tf
        BT
        1 0 0 -1 70.837906 78.400406 Tm
        [(??)] TJ
        ET
        "#);
        let list = helper![
            Number [1, 0, 0, -1, 0]
            Number [841.89105]
            Operator ["cm"]
            Key ["srgb"]
            Operator ["cs"]
            Number [0, 0, 0]
            Operator ["scn"]
            Key ["F0"]
            Number [11]
            Operator ["Tf", "BT"]
            Number [1, 0, 0, -1]
            Number [70.837906, 78.400406]
            Operator ["Tm"]
            ListStart $
            StringLiteral ["??"]
            ListEnd $
            Operator ["TJ", "ET"]
        ];

        assert_eq!(
            state.collect::<Vec<Token>>(),
            list
        );
    }

}