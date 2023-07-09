use std::collections::HashMap;
enum Value {
    Number(f64),
    String(String),
    Key(String),
    List(Vec<Value>),
    Ref(u8, u8),
    Dict(HashMap<String, Value>)
}

