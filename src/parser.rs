use std::{collections::HashMap, hash::Hash};
use super::lexer::{Token, self};
pub enum Value {
    Number(f64),
    String(String),
    Key(String),
    List(Vec<Value>),
    Ref(usize, usize),
    Dict(HashMap<String, Value>)
}

pub struct Object {
    id: (usize, usize),
    dict: HashMap<String, Value>,
    stream: Vec<u8>,
}

pub struct State {
    lexer: lexer::State,
    objects: HashMap<(usize, usize), Object>,
}

pub fn parse(source: &[u8]) -> Result<Vec<Object>, ()> {
    let mut state = State {
        lexer: lexer::parse(source),
        objects: HashMap::new(),
    };

    loop {
        let id = state.expect_obj_start()?;
        let dict = state.parse_dict()?;
        let mut stream = Vec::new();
        let next = state.lexer.next();
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
                    return Err(());
                };
                state.lexer.get_fixed_length_stream(len, &mut stream);
            }
            state.expect_stream_end()?;
        }
        state.expect_obj_end()?;     
        state.objects.insert(id, Object {
            id,
            dict,
            stream,
        });
    }
}

impl State {

    fn expect_obj_start(&mut self) -> Result<(usize, usize), ()> {
        if let Some(Token::ObjectStart(id)) = self.lexer.get_next_token() {
            return Ok(id)
        }
        Err(())
    }

    fn expect_obj_end(&mut self) -> Result<(), ()> {
        if let Some(Token::ObjectEnd) = self.lexer.get_next_token() {
            return Ok(())
        }
        Err(())
    }

    fn expect_stream_end(&mut self) -> Result<(), ()> {
        if let Some(Token::StreamEnd) = self.lexer.get_next_token() {
            return Ok(())
        }
        Err(())
    }

    fn parse_dict(&mut self) -> Result<HashMap<String, Value>, ()> {
        use Token::*;
        let mut ret = HashMap::new();
        loop {
            let token = self.lexer.next();
            if token == Some(DictEnd) {
                return Ok(ret);
            }
            let key = if let Some(Key(s)) = self.lexer.next() {
                s
            } else {
                return Err(());
            };
            let value = self.parse_value()?;
            ret.insert(key, value);
        }
    }

    fn parse_value(&mut self) -> Result<Value, ()> {
        use Token::*;
        let token = if let Some(x) = self.lexer.next() {
            x
        } else {
            return Err(());
        };
        match token {
            StringLiteral(s) => return Ok(Value::String(s)),
            Key(s) => return Ok(Value::Key(s)),
            StringLiteral(s) => return Ok(Value::String(s)),
            Ref((major, version)) => return Ok(Value::Ref(major, version)),
            Number(n) => return Ok(Value::Number(n)),
            DictEnd | ListEnd | StreamStart | StreamEnd | ObjectStart(..) | ObjectEnd => return Err(()),
            DictStart => self.parse_dict().map(Value::Dict),
            ListStart => {
                let mut ret = Vec::new();
                loop {
                    let token = self.lexer.next();
                    if token == Some(ListEnd) {
                        return Ok(Value::List(ret));
                    }
                    let value = self.parse_value()?;
                    ret.push(value);
                }
            }
        }
    }

}
