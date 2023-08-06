fn print(s: &[u8]) {
    let mut first = true;
    for &x in s {
        if first {
            first = false;
        } else {
            print!(", ");
        }
        if x.is_ascii() {
            print!("{}", x as char);
        } else {
            print!(" ");
        }
        print!("({:02x})", x);
    }
}

fn main() {
    use std::io::{Write, stdout};
    use pdf_parser::parser::{parse, Object, Value};
    use std::collections::HashMap;

    let pdf = parse(include_bytes!("../test.pdf")).unwrap();

    println!("meta {:?}", pdf.get_meta());

    println!("pages {:?}", pdf.get_pages().map(Object::dict));
    let kids = pdf.get_pages_kids().into_iter().flatten();
    
    for k in kids {
        println!("kid {:?}", k.dict());
        if let Some(&Value::Ref(m, n)) = k.dict().get("Contents") {
            println!("{:?}", pdf.get_objects().get(&(m, n)).unwrap().dict());
        }
    }

    println!("contents id {:?}", pdf.get_contents_id());
    for c in pdf.get_contents() {
        println!("content {:?}", c);
        stdout().write(c).unwrap();
        println!();
        // if c.iter().all(u8::is_ascii) {
        //     for &c in c {
        //         print!("{}", c as char);
        //     }
        //     println!();
        // } else {
        //     print(c);
        //     println!();
        // }
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

    for (id, k, v) in pdf.get_references() {
        println!("{:?} -> {} -> {:?}", id, k, v);
    }

    for (k, v) in pdf.get_objects() {
        use std::fs::File;
        // print(v.stream());
        if v.stream().is_empty() {
            continue;
        }
        println!("{k:?}");
        println!("{:?}", v.dict());
        let mut f = File::create(&format!("{k:?}.bin")).unwrap();
        f.write_all(v.stream()).unwrap();
    }

    let lines = pdf.get_cmaps_lines();

    let mut line_iter = lines.iter().map(|x| x.split("\n")).flatten();

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

                println!("{proxy_char} -> {target}");
            }
            break;
        }
    }
}
