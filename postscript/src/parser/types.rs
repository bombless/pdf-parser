pub enum Type {
    Any,
    Primitive(Primitive),
    Compound(Compound),
}

pub enum Primitive {
    Integer,
    Float,
    Key,
    String,
    Operator,
}

pub struct Compound {
    name: String,
    generics: Vec<Type>,
}
