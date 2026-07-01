//! A standalone QuarkGUI app with no dependency on quark-core — proof that the
//! toolkit is reusable on its own. Run with the headless backend so it works
//! without a display:
//!
//! ```text
//! cargo run -p quark-gui --example standalone --no-default-features
//! ```
//!
//! With the default Slint backend (`cargo run -p quark-gui --example standalone`)
//! it opens a real window instead.

use quark_gui::model::{Field, FormOutcome, FormSpec, Theme, WindowSpec};
use quark_gui::{App, Backend};

fn main() {
    // Auto picks Slint when compiled in, else Headless.
    let app = App::new(Backend::Auto);
    println!("QuarkGUI backend in use: {:?}", app.backend());

    let mut form = FormSpec::new(WindowSpec::new("QuarkGUI Demo", Theme::Light));
    form.submit_label = "Greet".into();
    form.fields = vec![
        Field::Text {
            id: "name".into(),
            label: "Your name".into(),
            value: "world".into(),
        },
        Field::Combo {
            id: "greeting".into(),
            label: "Greeting".into(),
            options: vec!["Hello".into(), "Hi".into(), "Hey".into()],
            selected: 0,
        },
        Field::Check {
            id: "loud".into(),
            label: "Shout it".into(),
            value: false,
        },
    ];

    match app.run_form(form) {
        FormOutcome::Submit(values) => {
            let greetings = ["Hello", "Hi", "Hey"];
            let greeting = greetings[values.index("greeting").min(2)];
            let mut message = format!("{greeting}, {}!", values.text("name"));
            if values.bool("loud") {
                message = message.to_uppercase();
            }
            println!("{message}");
        }
        FormOutcome::Button(id, _) => println!("button: {id}"),
        FormOutcome::Cancel => println!("cancelled"),
    }
}
