use std::collections::HashMap;
use super::lexer::{Token, self};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Key(String),
    List(Vec<Value>),
    Ref(usize, usize),
    Dict(HashMap<String, Value>)
}

pub struct Object {
    pub id: (usize, usize),
    pub dict: HashMap<String, Value>,
    pub stream: Vec<u8>,
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Object({:?}, {} keys, stream length {})\n", self.id, self.dict.len(), self.stream.len())
    }
}

pub struct State {
    lexer: lexer::State,
    objects: HashMap<(usize, usize), Object>,
}

pub fn parse(source: &[u8]) -> Result<HashMap<(usize, usize), Object>, String> {
    let mut state = State {
        lexer: lexer::parse(source),
        objects: HashMap::new(),
    };

    loop {

        if state.lexer.is(Token::XRef) {
            while let Some(line) = state.lexer.get_ascii_line() {
                println!("{line}");
            }
            return Ok(state.objects);
        }

        let id = state.expect_obj_start()?;
        let dict = state.parse_dict()?;
        let mut stream = Vec::new();
        let next = state.next_token();
        if next == Some(Token::StreamStart) {
            let kind = dict.get("Filter");
            let is_encoded = match kind {
                Some(Value::String(x)) if x == "FlateDecode" => true,
                _ => false,
            };
            if is_encoded {
                state.lexer.get_flate_stream(&mut stream);
            } else {
                let len = if let Some(Value::Number(n)) = dict.get("Length") {
                    *n as _
                } else {
                    return Err("where's .. length?".into());
                };
                state.lexer.get_fixed_length_stream(len, &mut stream);
            }
            state.expect_stream_end()?;
            state.expect_obj_end()?;
        }
        else if next != Some(Token::ObjectEnd) {
            return Err(format!("unexpected {next:?}"));
        }

        // println!("object {:?} parsed {:?}", id, dict);

        state.objects.insert(id, Object {
            id,
            dict,
            stream,
        });

    }
}

impl State {

    fn next_token(&mut self) -> Option<Token> {
        self.lexer.next()
    }

    fn swallow_token(&mut self, t: Token) {
        self.lexer.swallow(t);
    }

    fn expect_obj_start(&mut self) -> Result<(usize, usize), String> {
        let token = self.next_token();
        if let Some(Token::ObjectStart(id)) = token {
            return Ok(id)
        }
        Err(format!("expected ObjStart, found {token:?}"))
    }

    fn expect_obj_end(&mut self) -> Result<(), String> {
        if let Some(Token::ObjectEnd) = self.next_token() {
            return Ok(())
        }
        Err("expected ObjEnd".into())
    }

    fn expect_stream_end(&mut self) -> Result<(), String> {
        if let Some(Token::StreamEnd) = self.next_token() {
            return Ok(())
        }
        Err("expected StreamEnd".into())
    }

    fn expect_dict_start(&mut self) -> Result<(), String> {
        if let Some(Token::DictStart) = self.next_token() {
            return Ok(())
        }
        Err("expected DictStart".into())
    }

    fn parse_dict(&mut self) -> Result<HashMap<String, Value>, String> {
        use Token::*;
        let mut ret = HashMap::new();
        self.expect_dict_start()?;
        loop {
            let token = self.next_token();
            if token == Some(DictEnd) {
                return Ok(ret);
            }
            let key = if let Some(Key(s)) = token {
                s
            } else {
                return Err("expected Key".into());
            };
            let value = self.parse_value()?;
            ret.insert(key, value);
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        use Token::*;
        let token = if let Some(x) = self.next_token() {
            x
        } else {
            return Err("expected token".into());
        };
        match token {
            StringLiteral(s) => return Ok(Value::String(s)),
            Key(s) => return Ok(Value::Key(s)),
            Ref((major, version)) => return Ok(Value::Ref(major, version)),
            Number(n) => return Ok(Value::Number(n)),
            x @(DictEnd | ListEnd | StreamStart | StreamEnd | ObjectStart(..) | ObjectEnd | XRef) =>
                panic!("unexpected {x}"),
            DictStart => {
                self.swallow_token(DictStart);
                self.parse_dict().map(Value::Dict)
            }
            ListStart => {
                let mut ret = Vec::new();
                loop {
                    let token = if let Some(x) = self.next_token() {
                        x
                    } else {
                        return Err("unexpected EOF".into());
                    };
                    if token == ListEnd {
                        return Ok(Value::List(ret));
                    }
                    // println!("list parsed {:?}", ret);
                    self.swallow_token(token);
                    let value = self.parse_value()?;
                    ret.push(value);
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_list() {
        let mut state = State {
            lexer: lexer::parse(b"[1]"),
            objects: HashMap::new(),
        };
        assert_eq!(state.parse_value().unwrap(), Value::List(vec![Value::Number(1.0)]))
    }
    #[test]
    fn parse_dict() {
        let mut state = State {
            lexer: lexer::parse(b"<< /Value 42 >>"),
            objects: HashMap::new(),
        };
        let value = state.parse_value().unwrap();
        let value = match value { Value::Dict(d) => d, _ => panic!() };
        assert_eq!(value.len(), 1);
        assert_eq!(value.into_iter().next().unwrap(), ("Value".into(), Value::Number(42.)));
    }
    #[test]
    fn parse_mix() {
        let mut state = State {
            lexer: lexer::parse(b"<< /a [4 0 R] /b 6 0 R >>"),
            objects: HashMap::new(),
        };
        let value = state.parse_value().unwrap();
        let dict = match value { Value::Dict(d) => d, _ => panic!() };
        assert_eq!(dict.len(), 2);
        let mut iter = dict.into_iter();
        assert_eq!(iter.next().unwrap(), ("a".into(), Value::List(vec![Value::Ref(4, 0)])));
        assert_eq!(iter.next().unwrap(), ("a".into(), Value::Ref(6, 0)));
    }
}
