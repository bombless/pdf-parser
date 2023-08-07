use std::collections::HashMap;
use super::lexer::{Token, self};
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Key(String),
    List(Vec<Value>),
    Ref(usize, usize),
    Dict(HashMap<String, Value>),
    Id(u128),
    Null,
    Bool(bool),
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        match self {
            Value::Key(s) | Value::String(s) if s == other => true,
            _ => false
        }
    }
}

pub struct Object {
    id: (usize, usize),
    value: Value,
    stream: Vec<u8>,
}

lazy_static! {
    static ref DUMMY: HashMap<String, Value> = HashMap::new();
}

impl Object {
    pub fn dict(&self) -> &HashMap<String, Value> {
        if let &Value::Dict(ref dict) = &self.value {
            dict
        } else {
            &DUMMY
        }
    }

    pub fn id(&self) -> (usize, usize) {
        self.id
    }
    pub fn stream(&self) -> &[u8] {
        &self.stream
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = if let Some(Value::Key(s)) = self.dict().get("Type") {
            s.to_owned()
        } else {
            "".into()
        };
        write!(f, "Object({:?}, {name:?}, {} keys, stream length {})", self.id, self.dict().len(), self.stream.len())
    }
}

pub struct State {
    lexer: lexer::State,
}

pub struct PDF {
    objects: HashMap<(usize, usize), Object>,
    meta: HashMap<String, Value>,
}

impl PDF {
    pub fn get_objects(&self) -> &HashMap<(usize, usize), Object> {
        &self.objects
    }
    pub fn get_meta(&self) -> &HashMap<String, Value> {
        &self.meta
    }
    pub fn get_references(&self) -> Vec<((usize, usize), String, &Object)> {
        let mut ret = Vec::new();
        self.get_references_from_dict(&self.meta, (0, 0), &mut ret);
        for (&id, o) in &self.objects {
            for (k, v) in o.dict() {
                match v {
                    &Value::Ref(m, n) => {
                        ret.push((id, k.to_string(), self.objects.get(&(m, n)).unwrap()));
                    }
                    Value::Dict(dict) => {
                        self.get_references_from_dict(dict, id, &mut ret);
                    }
                    Value::List(l) => {
                        self.get_references_from_list(l, format!("{}[]", k), id, &mut ret)
                    }
                    _ => {}
                }
            }
        }
        ret
    }
    fn get_references_from_dict<'a>(&'a self, dict: &'a HashMap<String, Value>, object_id: (usize, usize), buf: &mut Vec<((usize, usize), String, &'a Object)>) {
        for (k, v) in dict {
            match v {
                &Value::Ref(m, n) => {
                    buf.push((object_id, k.to_string(), self.objects.get(&(m, n)).unwrap()));
                }
                Value::Dict(dict) => {
                    self.get_references_from_dict(dict, object_id, buf);
                }
                Value::List(l) => {
                    self.get_references_from_list(l, format!("{}[]", k), object_id, buf)
                }
                _ => {}
            }
        }

    }
    fn get_references_from_list<'a, 'b>(&'a self, list: &'a [Value], name: String, object_id: (usize, usize), buf: &mut Vec<((usize, usize), String, &'a Object)>) {
        for v in list {
            match v {
                &Value::Ref(m, n) => {
                    buf.push((object_id, name.clone(), self.objects.get(&(m, n)).unwrap()));
                }
                Value::Dict(dict) => {
                    self.get_references_from_dict(dict, object_id, buf);
                }
                Value::List(l) => {
                    self.get_references_from_list(l, name.clone(), object_id, buf)
                }
                _ => {}
            }
        }

    }
    pub fn get_pages(&self) -> Option<&Object> {
        let root = if let Some(&Value::Ref(major, minor)) = self.meta.get("Root") {
            (major, minor)
        } else {
            return None;
        };
        let root = self.objects.get(&root).unwrap();
        let pages = if let Some(&Value::Ref(major, minor)) = root.dict().get("Pages") {
            (major, minor)
        } else {
            return None;
        };
        self.objects.get(&pages)
    }

    pub fn get_pages_kids(&self) -> Option<Vec<&Object>> {
        let kids = if let Some(x) = self.get_pages() {
            if let Some(Value::List(list)) = x.dict().get("Kids") {
                list
            } else {
                return None;
            }
        } else {
            return None;
        };

        let mut iter = kids.into_iter();

        let mut ret = Vec::new();

        while let Some(x) = iter.next() {
            if let &Value::Ref(major, minor) = x {
                ret.push(self.objects.get(&(major, minor))?);
            } else {
                return None;
            }
        }

        Some(ret)

    }

    pub fn get_pages_grand_kids(&self) -> Option<Vec<&Object>> {
        let kids = if let Some(x) = self.get_pages_kids() {
            let mut vec = Vec::new();
            for item in x {
                if let Some(Value::List(list)) = item.dict().get("Kids") {
                    vec.extend(list)
                } else {
                    return None;
                }
            }
            vec
        } else {
            return None;
        };

        let mut iter = kids.into_iter();

        let mut ret = Vec::new();

        while let Some(x) = iter.next() {
            if let &Value::Ref(major, minor) = x {
                ret.push(self.objects.get(&(major, minor))?);
            } else {
                return None;
            }
        }

        Some(ret)

    }

    pub fn get_contents(&self) -> Vec<&[u8]> {
        let contents = if let Some(x) = self.get_pages_kids() {
            x
        } else {
            return Vec::new();
        };
        let mut ret = Vec::new();
        for x in contents {
            let c = if let Some(&Value::Ref(m, n)) = x.dict().get("Contents") {
                if let Some(x) = self.objects.get(&(m, n)) {
                    &x.stream[..]
                } else {
                    continue;
                }
            } else {
                continue;
            };
            ret.push(c);
        }
        ret
    }

    pub fn get_contents_id(&self) -> Vec<(usize, usize)> {
        let contents = if let Some(x) = self.get_pages_kids() {
            x
        } else {
            return Vec::new();
        };
        let mut ret = Vec::new();
        for x in contents {
            let id = if let Some(&Value::Ref(m, n)) = x.dict().get("Contents") {
                (m, n)
            } else {
                continue;
            };
            ret.push(id);
        }
        ret
    }

    pub fn get_fonts(&self) -> HashMap<&str, &Object> {
        let pages = if let Some(x) = self.get_pages() {
            x.dict()
        } else {
            return HashMap::new();
        };
        let resources = if let Some(Value::Dict(x)) = pages.get("Resources") {
            x
        } else {
            return HashMap::new();
        };
        let fonts = resources.get("Font");
        if let Some(Value::Dict(x)) = fonts {
            let mut ret = HashMap::new();
            for (k, v) in x {
                let r = if let &Value::Ref(m, n) = v {
                    let r = self.objects.get(&(m, n));
                    match r {
                        Some(x) => x,
                        None => continue,
                    }
                } else {
                    continue;
                };
                ret.insert(&**k, r);
            }
            return ret;
        }
        HashMap::new()
    }

    pub fn get_cmaps(&self) -> Vec<&[u8]> {
        let mut ret = Vec::new();
        for (_, obj) in &self.objects {
            if obj.dict().get("Type").map_or(false, |x| x == "CMap") {
                ret.push(&obj.stream[..]);
            }
        }
        ret
    }

    pub fn get_cmaps_lines(&self) -> Vec<String> {
        let mut ret = Vec::new();
        for (_, obj) in &self.objects {
            if obj.dict().get("Type").map_or(false, |x| x == "CMap") {
                if obj.stream.is_ascii() {
                    ret.push(String::from_utf8(obj.stream.clone()).unwrap());
                }
            }
        }
        ret
    }

    pub fn get_descendant_fonts(&self) -> Vec<&Object> {
        let mut ret = Vec::new();
        for (_, f) in self.get_fonts() {
            let dict = f.dict();
            if let Some(Value::List(f)) = dict.get("DescendantFonts") {
                for x in f {
                    if let &Value::Ref(m, n) = x {
                        if let Some(x) = self.objects.get(&(m, n)) {
                            ret.push(x);
                        }
                    }
                }
            }
        }
        ret
    }

    pub fn get_font_describtors(&self) -> Vec<&Object> {
        let mut ret = Vec::new();
        for o in self.get_descendant_fonts() {
            let dict = o.dict();
            if let Some(&Value::Ref(m, n)) = dict.get("FontDescriptor") {
                if let Some(x) = self.objects.get(&(m, n)) {
                    ret.push(x);
                }
            }
        }
        ret
    }

    pub fn get(&self, id: &(usize, usize)) -> Option<&Object> {
        self.objects.get(id)
    }
}

pub fn parse(source: &[u8]) -> Result<PDF, String> {

    let mut state = State {
        lexer: lexer::parse(source),        
    };

    let mut objects = HashMap::new();

    loop {

        if state.lexer.is(Token::XRef) {
            state.lexer.next();
            while let Some(line) = state.lexer.get_ascii_line() {
                if line == "trailer" {
                    let meta = state.parse_dict()?;
                    let pdf = PDF {
                        meta,
                        objects,
                    };
                    
                    return Ok(pdf);
                }
            }
            return Err("Something wrong".into());
        }

        let id = state.expect_obj_start()?;
        let value = state.parse_value()?;
        let dict = if let &Value::Dict(ref dict) = &value {
            dict
        } else {
            &DUMMY
        };
        let mut stream = Vec::new();
        let next = state.next_token();
        if next == Some(Token::StreamStart) {
            let kind = dict.get("Filter");
            let is_encoded = match kind {
                Some(Value::Key(x)) if x == "FlateDecode" => true,
                _ => false,
            };
            let len = if let Some(Value::Number(n)) = dict.get("Length") {
                *n as _
            } else {
                return Err("where's .. length?".into());
            };
            if is_encoded {
                state.lexer.get_flate_stream(len, &mut stream);
            } else {                
                state.lexer.get_fixed_length_stream(len, &mut stream);
            }
            state.expect_stream_end()?;
            state.expect_obj_end()?;
        }
        else if next != Some(Token::ObjectEnd) {
            return Err(format!("unexpected {next:?}"));
        }


        objects.insert(id, Object {
            id,
            value,
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
        let t = self.next_token();
        if let Some(Token::DictStart) = t {
            return Ok(())
        }
        Err(format!("expected DictStart, got {t:?}"))
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
                return Err(format!("expected Key, got {token:?}"));
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
            Bool(b) => return Ok(Value::Bool(b)),
            Null => return Ok(Value::Null),
            Eof => return Err("unexpected EOF".into()),
            Id(x) => return Ok(Value::Id(x)),
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
                        return Err("unexpected lexing error".into());
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
    fn test() {
        parse(include_bytes!("../dict.dump")).unwrap();
    }

    #[test]
    fn parse_list() {
        let mut state = State {
            lexer: lexer::parse(b"[1]"),
        };
        assert_eq!(state.parse_value().unwrap(), Value::List(vec![Value::Number(1.0)]))
    }
    #[test]
    fn parse_trailer() {
        let mut state = State {
            lexer: lexer::parse(b"<<
            /Size 12
            /Root 11 0 R
            /Info 9 0 R
          >>"),
        };
        let value = state.parse_value().unwrap();
        let value = match value { Value::Dict(d) => d, _ => panic!() };
        assert_eq!(value.len(), 3);
    }
    #[test]
    fn parse_dict() {
        let mut state = State {
            lexer: lexer::parse(b"<< /Value 42 >>"),
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
        };
        let value = state.parse_value().unwrap();
        let dict = match value { Value::Dict(d) => d, _ => panic!() };
        assert_eq!(dict.len(), 2);
        let mut iter = dict.into_iter();
        assert_eq!(iter.next().unwrap(), ("a".into(), Value::List(vec![Value::Ref(4, 0)])));
        assert_eq!(iter.next().unwrap(), ("a".into(), Value::Ref(6, 0)));
    }
}
