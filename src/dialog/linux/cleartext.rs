use crate::prompt::DialogResult;
use crate::security::disable_core_dumps;
use std::cell::RefCell;
use std::rc::Rc;
use zeroize::Zeroizing;

slint::slint! {
    import { Button, CheckBox, LineEdit } from "std-widgets.slint";

    export component ClearTextDialog inherits Window {
        in property <string> prompt-text;
        in property <string> checkbox-label;
        in property <bool> show-checkbox;
        in-out property <string> value;
        in-out property <bool> save-checked;
        callback ok();
        callback cancel();

        title: "SSH";
        width: 380px;
        height: layout.preferred-height;
        forward-focus: input;

        layout := VerticalLayout {
            padding: 16px;
            spacing: 12px;

            Text {
                text: prompt-text;
                wrap: word-wrap;
            }

            input := LineEdit {
                text <=> value;
                height: 32px;
                accepted => { ok(); }
            }

            if show-checkbox : CheckBox {
                text: checkbox-label;
                checked <=> save-checked;
            }

            HorizontalLayout {
                alignment: end;
                spacing: 8px;

                Button {
                    text: "Cancel";
                    clicked => { cancel(); }
                }
                Button {
                    text: "OK";
                    primary: true;
                    clicked => { ok(); }
                }
            }
        }
    }
}

pub fn show(
    prompt: &str,
    identifier: Option<&str>,
    store: &dyn crate::store::SecretStore,
) -> DialogResult {
    disable_core_dumps();

    let dialog = ClearTextDialog::new().unwrap();
    dialog.set_prompt_text(prompt.into());

    let show_checkbox = identifier.is_some();
    dialog.set_show_checkbox(show_checkbox);
    if show_checkbox {
        let overwrite = identifier.is_some_and(|id| store.exists(id));
        dialog.set_checkbox_label(
            if overwrite {
                "Overwrite stored password"
            } else {
                "Remember in keyring"
            }
            .into(),
        );
    }

    let accepted = Rc::new(RefCell::new(false));

    let accepted_ok = accepted.clone();
    let weak_ok = dialog.as_weak();
    dialog.on_ok(move || {
        *accepted_ok.borrow_mut() = true;
        if let Some(d) = weak_ok.upgrade() {
            let _ = d.hide();
        }
    });

    let weak_cancel = dialog.as_weak();
    dialog.on_cancel(move || {
        if let Some(d) = weak_cancel.upgrade() {
            let _ = d.hide();
        }
    });

    dialog.run().unwrap();

    if *accepted.borrow() {
        let s = dialog.get_value().to_string();
        let save_secret = show_checkbox && dialog.get_save_checked();
        drop(dialog);

        DialogResult::Accepted {
            secret: Zeroizing::new(s),
            save_secret,
        }
    } else {
        drop(dialog);
        DialogResult::Cancelled
    }
}
