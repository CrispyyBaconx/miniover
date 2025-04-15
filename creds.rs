use native_windows_gui::{self as nwg, NativeUi};
use std::rc::Rc;
use std::cell::RefCell;
use tokio::sync::oneshot;

#[derive(Default)]
pub struct LoginDialog {
    window: nwg::Window,
    layout: nwg::GridLayout,
    
    email_label: nwg::Label,
    email_input: nwg::TextInput,
    
    password_label: nwg::Label,
    password_input: nwg::TextInput,
    
    login_button: nwg::Button,
    cancel_button: nwg::Button,
    
    result_sender: RefCell<Option<oneshot::Sender<Option<(String, String)>>>>,
}

pub struct LoginDialogUi {
    inner: Rc<LoginDialog>,
    default_handler: RefCell<Option<nwg::EventHandler>>,
}

impl LoginDialog {
    fn submit(&self) {
        let email = self.email_input.text();
        let password = self.password_input.text();
        
        // Send the result back through the channel
        if let Some(sender) = self.result_sender.borrow_mut().take() {
            let _ = sender.send(Some((email, password)));
        }
        
        self.window.close();
    }
    
    fn cancel(&self) {
        // Send None to indicate cancellation
        if let Some(sender) = self.result_sender.borrow_mut().take() {
            let _ = sender.send(None);
        }
        
        self.window.close();
    }
    
    // Set the sender for the result
    pub fn set_result_sender(&self, sender: oneshot::Sender<Option<(String, String)>>) {
        *self.result_sender.borrow_mut() = Some(sender);
    }
}

impl nwg::NativeUi<LoginDialogUi> for LoginDialog {
    fn build_ui(mut data: LoginDialog) -> Result<LoginDialogUi, nwg::NwgError> {
        // Controls
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .size((300, 150))
            .position((300, 300))
            .title("Login")
            .build(&mut data.window)?;
            
        nwg::Label::builder()
            .text("Email:")
            .parent(&data.window)
            .build(&mut data.email_label)?;
            
        nwg::TextInput::builder()
            .parent(&data.window)
            .build(&mut data.email_input)?;
            
        nwg::Label::builder()
            .text("Password:")
            .parent(&data.window)
            .build(&mut data.password_label)?;
            
        nwg::TextInput::builder()
            .parent(&data.window)
            .flags(nwg::TextInputFlags::VISIBLE)
            .build(&mut data.password_input)?;
            
        nwg::Button::builder()
            .text("Login")
            .parent(&data.window)
            .build(&mut data.login_button)?;
            
        nwg::Button::builder()
            .text("Cancel")
            .parent(&data.window)
            .build(&mut data.cancel_button)?;
            
        // Layout
        nwg::GridLayout::builder()
            .parent(&data.window)
            .spacing(1)
            .child(0, 0, &data.email_label)
            .child(1, 0, &data.email_input)
            .child(0, 1, &data.password_label)
            .child(1, 1, &data.password_input)
            .child(0, 2, &data.login_button)
            .child(1, 2, &data.cancel_button)
            .build(&data.layout)?;

        // Wrap-up
        let ui = LoginDialogUi {
            inner: Rc::new(data),
            default_handler: Default::default(),
        };

        // Events
        let event_ui = Rc::downgrade(&ui.inner);
        let handle_events = move |evt, _evt_data, handle| {
            if let Some(ui) = event_ui.upgrade() {
                match evt {
                    nwg::Event::OnButtonClick => {
                        if &handle == &ui.login_button {
                            ui.submit();
                        } else if &handle == &ui.cancel_button {
                            ui.cancel();
                        }
                    }
                    _ => {}
                }
            }
        };

        *ui.default_handler.borrow_mut() = Some(nwg::full_bind_event_handler(&ui.inner.window.handle, handle_events));
            
        Ok(ui)
    }
}

impl Drop for LoginDialogUi {
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if let Some(handler) = handler.as_ref() {
            nwg::unbind_event_handler(handler);
        }
    }
}

// Function to show the dialog and get credentials asynchronously
pub async fn get_credentials() -> Option<(String, String)> {
    let (tx, rx) = oneshot::channel();
    
    // Spawn a thread to run the GUI since it has its own event loop
    std::thread::spawn(move || {
        nwg::init().expect("Failed to init Native Windows GUI");
        
        let app = LoginDialog::default();
        app.set_result_sender(tx);
        
        let _ui = LoginDialog::build_ui(app).expect("Failed to build UI");
        
        nwg::dispatch_thread_events();
    });
    
    // Wait for the result from the GUI thread
    rx.await.unwrap_or(None)
}