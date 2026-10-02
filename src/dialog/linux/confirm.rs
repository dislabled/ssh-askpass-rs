use crate::prompt::DialogResult;
use crate::security::disable_core_dumps;
use std::cell::RefCell;
use std::rc::Rc;

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component NoticeDialog inherits Window {
        in property <string> prompt-text;
        callback dismiss();

        title: "SSH";
        width: 380px;
        height: layout.preferred-height;
        forward-focus: ok-button;

        layout := VerticalLayout {
            padding: 16px;
            spacing: 12px;

            Text {
                text: prompt-text;
                wrap: word-wrap;
            }

            HorizontalLayout {
                alignment: end;

                ok-button := Button {
                    text: "OK";
                    primary: true;
                    clicked => { dismiss(); }
                }
            }
        }
    }

    export component YesNoDialog inherits Window {
        in property <string> prompt-text;
        callback yes();
        callback dismiss();

        title: "SSH";
        width: 380px;
        height: layout.preferred-height;
        forward-focus: no-button;

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

                // Default, and receives initial keyboard focus
                no-button := Button {
                    text: "No";
                    primary: true;
                    clicked => { dismiss(); }
                }
                Button {
                    text: "Yes";
                    clicked => { yes(); }
                }
            }
        }
    }
}

/// Ask a plain yes/no question
pub fn ask_retry(message: &str) -> bool {
    disable_core_dumps();

    let dialog = YesNoDialog::new().unwrap();
    dialog.set_prompt_text(message.into());

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

    let result = *accepted.borrow();
    result
}

pub fn show(prompt: &str, cancel_only: bool) -> DialogResult {
    disable_core_dumps();

    if cancel_only {
        let dialog = NoticeDialog::new().unwrap();
        dialog.set_prompt_text(prompt.into());

        let weak_dismiss = dialog.as_weak();
        dialog.on_dismiss(move || {
            if let Some(d) = weak_dismiss.upgrade() {
                let _ = d.hide();
            }
        });

        dialog.run().unwrap();
        DialogResult::Cancelled
    } else {
        let dialog = YesNoDialog::new().unwrap();
        dialog.set_prompt_text(prompt.into());

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

        if *accepted.borrow() {
            DialogResult::yes()
        } else {
            DialogResult::Cancelled
        }
    }

