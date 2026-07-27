use crate::security::disable_core_dumps;
use std::cell::RefCell;
use std::rc::Rc;

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AutofillDialog inherits Window {
        in property <string> body-text;
        callback send();
        callback dont-send();

        title: "Send stored credential?";
        width: 420px;
        height: layout.preferred-height;
        forward-focus: dont-send-button;

        layout := VerticalLayout {
            padding: 16px;
            spacing: 12px;

            Text {
                text: body-text;
                wrap: word-wrap;
            }

            HorizontalLayout {
                alignment: end;
                spacing: 8px;

                // Default, and receives initial keyboard focus
                dont-send-button := Button {
                    text: "Don't Send";
                    primary: true;
                    clicked => { dont-send(); }
                }
                Button {
                    text: "Send";
                    clicked => { send(); }
                }
            }
        }
    }
}

pub fn confirm_autofill(prompt: &str, identifier: &str) -> bool {
    disable_core_dumps();

    let dialog = AutofillDialog::new().unwrap();
    let body = format!(
        "A request is asking for:\n  {}\n\nStored credential to send:\n  {}",
        prompt.trim_end(),
        identifier
    );
    dialog.set_body_text(body.into());

    let accepted = Rc::new(RefCell::new(false));

    let accepted_send = accepted.clone();
    let weak_send = dialog.as_weak();
    dialog.on_send(move || {
        *accepted_send.borrow_mut() = true;
        if let Some(d) = weak_send.upgrade() {
            let _ = d.hide();
        }
    });

    let weak_dont_send = dialog.as_weak();
    dialog.on_dont_send(move || {
        if let Some(d) = weak_dont_send.upgrade() {
            let _ = d.hide();
        }
    });

    dialog.run().unwrap();

    let result = *accepted.borrow();
    result
}
