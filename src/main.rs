use postscript::lexer::parse as lexer;
use postscript::parser::*;
use std::collections::HashMap;

mod cli;

fn main() {
    use std::fs::File;
    use std::io::Read;
    use pdf_parser::parser::parse;

    let options = cli::parse_options();

    let file_path = options.get_one::<String>("FILE").expect("Require file name");
    let mut file = File::open(file_path).unwrap();
    let mut content = Vec::new();
    file.read_to_end(&mut content).unwrap();

    let pdf = parse(&content).unwrap();

    if options.get_flag("meta") {
        println!("meta {:?}", pdf.get_meta());
    }

    if options.get_flag("pages") {
        if let Some(x) = pdf.get_pages() {
            println!("{x:?}");
            println!("{:?}", x.dict());
        }

        for obj in pdf.get_pages_kids().unwrap() {
            println!("{obj:?}");
            println!("{:?}", obj.dict());
        }
    }

    if options.get_flag("grand_kids") {
        for x in pdf.get_pages_grand_kids().unwrap() {
            println!("grand kid {x:?} {:?}", x.dict());
        }
    }

    let lines = pdf.get_cmaps_lines();

    let mut line_iter = lines.iter().map(|x| x.split("\n")).flatten();

    let mut babel = HashMap::new();

    while let Some(v) = line_iter.next() {
        if v.ends_with(" beginbfchar") {
            let n: usize = v.split(" ").next().unwrap().parse().unwrap();
            for _ in 0 .. n {
                let line = line_iter.next().unwrap();
                if options.get_flag("cmap") {
                    println!("{line}");
                }
                let mut components = line.split("> <");
                let left = &components.next().unwrap()[1..];
                let right_half_bake = &components.next().unwrap();
                let right = &right_half_bake[..right_half_bake.len() - 1];

                if options.get_flag("cmap") {
                    println!("{left} -> {right}");
                }

                let proxy_char = u16::from_str_radix(left, 16).unwrap();
                let target = char::from_u32(u32::from_str_radix(right, 16).unwrap()).unwrap();

                babel.insert(proxy_char, target);

                if options.get_flag("cmap") {
                    println!("{proxy_char} -> {target}");
                }
            }
            break;
        }
    }

    if options.get_flag("babel") {
        println!("{babel:?}");
    }

    #[cfg(feature = "slint")]
    {
        use pdf_parser::text::handle_text_operation;
        use slint_ui::{TextItem, run};
        use pdf_parser::operation::TextState;
        use postscript::parser::parse;
        let first_page = pdf.get_first_page().unwrap();
        let mut texts = Vec::new();
        for obj in first_page {
            let stream = obj.stream();
            if stream.is_ascii() {
                println!("{}", String::from_utf8(stream.into()).unwrap());
            }
            let state = lexer(stream);
            let mut parser = parse(state).into_iter();
            while let Some(x) = parser.next() {
                println!("{x:?}");
                if x.op == "BT" {
                    let mut text_state = TextState::default();
                    while let Some(x) = parser.next() {
                        println!("{x:?}");
                        if x.op == "ET" {
                            break;
                        }
                        handle_text_operation(x, &mut text_state, &babel);
                    }
                    for op in text_state.drain() {
                        println!("{op:?}");
                        texts.push(TextItem {
                            x: op.x as _,
                            y: op.y as _,
                            size: op.font_size as _,
                            text: op.text.into(),
                        });
                    }
                }
            }
        }

        run(texts);
    }

    if options.get_flag("first_page") {
        let first_page = pdf.get_first_page().unwrap();
        for obj in first_page {
            println!("{obj:?}");
            println!("{:?}", obj.dict());
            let state = lexer(obj.stream());
            for line in collect_texts(state, &babel) {
                println!("{line}");
            }
        }
    }


    if options.get_flag("operations") {

        for (_, name, obj) in pdf.get_references() {
            if name != "Contents" {
                continue;
            }
            let lexer = lexer(obj.stream());
            for x in collect_operations(lexer) {
                println!("{x}");
            }
        }
    }

    if let Some(x) = options.get_one::<String>("nth") {
        if let Ok(n) = x.parse() {
            if let Some(obj) = pdf.get(&(n, 0)) {
                println!("{obj:?}");
                println!("{:?}", obj.dict());
                let stream = obj.stream();
                if stream.is_ascii() {
                    println!("{}", String::from_utf8(stream.into()).unwrap());
                } else {
                    println!("{stream:?}");
                }
            }
        }
    }

    if options.get_flag("all") {
        let mut vec = pdf.get_objects().values().collect::<Vec<_>>();
        vec.sort_by_key(|v| v.id());
        for v in vec {
            println!("{v:?}");
        }
    }

    if !options.get_flag("texts") {
        return;
    }



    for (.., obj) in pdf.get_references() {
        if !obj.dict().get("Type").map_or(false, |x| x == "Text") {
            continue;
        }
        let lexer = lexer(obj.stream());
        println!("Text {obj:?}");
        for line in collect_texts(lexer, &babel) {
            println!("{line}");
        }
    }


    for c in pdf.get_contents() {
        let lexer = lexer(c);
        for line in collect_texts(lexer, &babel) {
            println!("{line}");
        }
    }

    for (_, name, obj) in pdf.get_references() {
        if name != "Contents" {
            continue;
        }
        let lexer = lexer(obj.stream());
        println!("{obj:?}");
        for line in collect_texts(lexer, &babel) {
            println!("{line}");
        }
    }

}
