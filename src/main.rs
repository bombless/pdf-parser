fn main() {
    use pdf_parser::lexer::{parse, Token};
    let mut tokenize_state = parse(include_bytes!("../test.pdf"));
    while let Some(t) = tokenize_state.get_next_token() {
        println!("{:?}", t);
        if t == Token::StreamStart {
            break
        }
    }
    println!("index {}(0x{:x}) length {}", tokenize_state.index(), tokenize_state.index(), tokenize_state.len())
}
