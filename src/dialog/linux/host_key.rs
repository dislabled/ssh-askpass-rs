use crate::prompt::DialogResult;
use crate::security::disable_core_dumps;
use std::cell::RefCell;
use std::rc::Rc;

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component HostKeyDialog inherits Window {
        in property <string> prompt-text;
        callback yes();
        callback no();

        title: "Unknown SSH Host Key";
        width: 420px;
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
                    clicked => { no(); }
                }
                Button {
                    text: "Yes";
                    clicked => { yes(); }
                }
            }
        }
    }
}

pub fn show(prompt: &str) -> DialogResult {
    disable_core_dumps();

    let cleaned = prompt
        .replace("(yes/no/[fingerprint])", "")
        .replace("(yes/no)", "")
        .replace("Are you sure", "\nAre you sure")
        .trim()
        .to_string();

    let dialog = HostKeyDialog::new().unwrap();
    dialog.set_prompt_text(cleaned.into());

    let accepted = Rc::new(RefCell::new(false));

    let accepted_yes = accepted.clone();
    let weak_yes = dialog.as_weak();
    dialog.on_yes(move || {
        *accepted_yes.borrow_mut() = true;
        if let Some(d) = weak_yes.upgrade() {
            let _ = d.hide();
        }
    });

    let weak_no = dialog.as_weak();
    dialog.on_no(move || {
        if let Some(d) = weak_no.upgrade() {
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
