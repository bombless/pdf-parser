slint::slint! {
    export struct TextItem {
        x: length,
        y: length,
        size: length,
        text: string,
    }
    export component MainWindow inherits Window {
        in property <float> w;
        in property <float> h;
        in property <[TextItem]> model;
        width: w * 1pt;
        height: h * 1pt;
        for text in model: Text {
            x: text.x;
            y: text.y;
            font-size: text.size;
            text: text.text;
        }
    }
}

pub fn run(texts: Vec<TextItem>, window_size: (f64, f64)) {
    use slint::VecModel;
    use std::rc::Rc;
    let window = MainWindow::new().unwrap();
    window.set_w(window_size.0 as _);
    window.set_h(window_size.1 as _);
    window.set_model(Rc::new(VecModel::from(texts)).into());
    window.run().unwrap()
}
