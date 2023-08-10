slint::slint! {
    export struct TextItem {
        x: length,
        y: length,
        size: length,
        text: string,
    }
    export component MainWindow {
        in property <[TextItem]> model;
        for text in model: Text {
            x: text.x;
            y: text.y;
            font-size: text.size;
            text: text.text;
        }
    }
}

pub fn run(texts: Vec<TextItem>) {
    use slint::VecModel;
    use std::rc::Rc;
    let window = MainWindow::new().unwrap();
    window.set_model(Rc::new(VecModel::from(texts)).into());
    window.run().unwrap()
}
