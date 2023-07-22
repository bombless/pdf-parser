fn main() {
    use pdf_parser::parser::{parse, Object};
    let pdf = parse(include_bytes!("../test.pdf")).unwrap();
    println!("pages {:?}", pdf.get_pages().map(Object::dict));
    let kids = pdf.get_pages_kids().into_iter().flatten();
    
    for k in kids {
        println!("kid {:?}", k.dict());
    }
    for c in pdf.get_contents() {
        println!("content {:?}", c);
        if c.iter().all(u8::is_ascii) {
            for &c in c {
                print!("{}", c as char);
            }
            println!();
        }
    }

    for (name, obj) in pdf.get_fonts() {
        println!("font {name} {:?}", obj.dict());
    }
}
