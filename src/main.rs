use postscript::lexer::{Token::*, State};
use postscript::parser::collect;
use std::collections::HashMap;

fn main() {
    use pdf_parser::parser::{parse, Object, Value};

    let pdf = parse(include_bytes!("../attention.pdf")).unwrap();

    println!("meta {:?}", pdf.get_meta());

    println!("pages {:?}", pdf.get_pages().map(Object::dict));

    println!("pages kids {:?}", pdf.get_pages_kids());

    for x in pdf.get_pages_grand_kids().unwrap() {
        println!("grand kid {x:?} {:?}", x.dict());
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

    let lines = pdf.get_cmaps_lines();

    let mut line_iter = lines.iter().map(|x| x.split("\n")).flatten();

    let mut babel = HashMap::new();

    while let Some(v) = line_iter.next() {
        if v.ends_with(" beginbfchar") {
            let n: usize = v.split(" ").next().unwrap().parse().unwrap();
            for _ in 0 .. n {
                let line = line_iter.next().unwrap();
                println!("{line}");
                let mut components = line.split("> <");
                let left = &components.next().unwrap()[1..];
                let right_half_bake = &components.next().unwrap();
                let right = &right_half_bake[..right_half_bake.len() - 1];

                println!("{left} -> {right}");

                let proxy_char = u16::from_str_radix(left, 16).unwrap();
                let target = char::from_u32(u32::from_str_radix(right, 16).unwrap()).unwrap();

                babel.insert(proxy_char, target);
                println!("{proxy_char} -> {target}");
            }
            break;
        }
    }


    for c in pdf.get_contents() {
        let mut lexer = postscript::lexer::parse(c);
        while let Some(x) = lexer.next() {
            match x {
                Operator(op) if op == "BT" => {
                    parse_bt(lexer, &babel);
                    break;
                }
                _ => {}
            }
        }
    }

    for (_, name, obj) in pdf.get_references() {
        if name != "Contents" {
            continue;
        }
        let mut lexer = postscript::lexer::parse(obj.stream());
        while let Some(x) = lexer.next() {
            match x {
                Operator(op) if op == "BT" => {
                    parse_bt(lexer, &babel);
                    break;
                }
                _ => {}
            }
        }
    }

}

fn parse_bt(state: State, babel: &HashMap<u16, char>) {
    for line in collect(state, babel) {
        println!("{line}");
    }
}