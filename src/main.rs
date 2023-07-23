fn main() {
    use pdf_parser::parser::{parse, Object, Value};
    use std::collections::HashMap;

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
        println!("font {name} {:?} {:?}", obj.id(), obj.dict());
    }

    for entry in pdf.get_descendant_fonts() {
        println!("{entry:?}");
        println!("DescendantFonts {:?}", entry.dict());
    }

    let mut references = HashMap::new();

    for entry in pdf.get_font_describtors() {
        println!("{entry:?}");
        println!("FontDescriptor {:?}", entry.dict());
        for (k, v) in entry.dict() {
            if let Value::Ref(m, n) = v {
                if let Some(x) = pdf.get(&(*m, *n)) {
                    references.insert(k, x);
                }
            }
        }
    }

    for (name, v) in references {
        println!("{name} {v:?}\n{:?}", v.dict())
    }
}
