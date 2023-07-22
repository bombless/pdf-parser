fn main() {
    use pdf_parser::parser::{parse, Object};
    let pdf = parse(include_bytes!("../test.pdf")).unwrap();
    println!("pages {:?}", pdf.get_pages().map(Object::dict));
    let kids = pdf.get_pages_kids().into_iter().flatten();
    
    for k in kids {
        println!("kid {:?}", k.dict());
    }
}
