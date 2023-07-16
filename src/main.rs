fn main() {
    use pdf_parser::parser::parse;
    let structure = parse(include_bytes!("../test.pdf")).unwrap();
    for (id, obj) in structure {
        println!("{id:?}: {obj:?}");
    }
}
