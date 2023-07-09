fn main() {
    use pdf_parser::lexer::parse;
    let mut tokenizer = parse(include_bytes!("../test.pdf"));
    println!("{:?}", tokenizer.get_next_token())
}
