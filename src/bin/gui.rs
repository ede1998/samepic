use dioxus::prelude::{*, dioxus_elements::img};

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
        img{
            src: "samples/mama_1.jpg"
        }
    })
}