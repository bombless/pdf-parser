
#[allow(unused)]
enum TextStateOperator {
    TC, Tf, TL, Tr, Ts, Tw, Tz
}

#[allow(unused)]
enum TextPositioningOperator {
    Td, TD, Tm, TStar
}

#[allow(unused)]
enum TextPaintingOperator {
    Tj, TJ
}

#[derive(Debug)]
pub struct TextPaintingOperation {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub font_size: f64,
}

#[derive(Default)]
pub struct TextState {
    x: f64,
    y: f64,
    font_size: f64,
    paintings: Vec<TextPaintingOperation>,
}

impl TextState {
    pub fn set_font_size(&mut self, size: f64) {
        self.font_size = size;
    }
    pub fn get_font_size(&self) -> f64 {
        self.font_size 
    }
    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }
    pub fn get_pos(&self) -> (f64, f64) {
        (self.x, self.y)
    }
    pub fn push(&mut self, s: String) {
        self.paintings.push(TextPaintingOperation { x: self.x, y: self.y, text: s, font_size: self.font_size })
    }
    pub fn drain(&mut self) -> Vec<TextPaintingOperation> {
        self.paintings.drain(..).collect()
    }
}

#[allow(unused)]
struct Tm {
    a: f64, b: f64, c: f64, d: f64, e: f64, f: f64,
}
