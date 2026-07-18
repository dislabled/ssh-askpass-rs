use crate::prompt::DialogResult;
use crate::security::disable_core_dumps;
use std::cell::RefCell;
use std::rc::Rc;

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component ConfirmDialog inherits Window {
        in property <string> prompt-text;
        in property <bool> cancel-only;
        callback yes();
        callback dismiss();

        title: "SSH";
        width: 380px;
        height: layout.preferred-height;

        layout := VerticalLayout {
            padding: 16px;
            spacing: 12px;

            Text {
                text: prompt-text;
                wrap: word-wrap;
            }

            HorizontalLayout {
                alignment: end;
                spacing: 8px;

                if cancel-only : Button {
                    text: "OK";
                    primary: true;
                    clicked => { dismiss(); }
                }

                if !cancel-only : Button {
                    text: "No";
                    primary: true;
                    clicked => { dismiss(); }
                }
                if !cancel-only : Button {
                    text: "Yes";
                    clicked => { yes(); }
                }
            }
        }
    }
}

pub fn show(prompt: &str, cancel_only: bool) -> DialogResult {
    disable_core_dumps();

    let dialog = ConfirmDialog::new().unwrap();
    dialog.set_prompt_text(prompt.into());
    dialog.set_cancel_only(cancel_only);

    let accepted = Rc::new(RefCell::new(false));

    let accepted_yes = accepted.clone();
    let weak_yes = dialog.as_weak();
    dialog.on_yes(move || {
        *accepted_yes.borrow_mut() = true;
        if let Some(d) = weak_yes.upgrade() {
            let _ = d.hide();
        }
    });

    let weak_dismiss = dialog.as_weak();
    dialog.on_dismiss(move || {
        if let Some(d) = weak_dismiss.upgrade() {
            let _ = d.hide();
        }
    });

    dialog.run().unwrap();

    if !cancel_only && *accepted.borrow() {
        DialogResult::yes()
    } else {
        DialogResult::Cancelled
    }
}
