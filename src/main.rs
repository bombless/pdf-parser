use postscript::lexer::{parse as lexer, Token::*, State};
use postscript::parser::*;
use std::collections::HashMap;

mod cli;

fn main() {
    use pdf_parser::parser::parse;

    let options = cli::parse_options();


    let pdf = parse(include_bytes!("../attention.pdf")).unwrap();

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
