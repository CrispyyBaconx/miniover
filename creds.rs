// ============================================================================
// Windows implementation using native-windows-gui
// ============================================================================

#[cfg(windows)]
use tokio::sync::oneshot;
#[cfg(windows)]
use native_windows_gui::{self as nwg, NativeUi};
#[cfg(windows)]
use std::rc::Rc;
#[cfg(windows)]
use std::cell::RefCell;

#[cfg(windows)]
#[derive(Default)]
pub struct LoginDialog {
    window: nwg::Window,
    layout: nwg::GridLayout,
    
    text_box: nwg::TextBox,
    
    email_label: nwg::Label,
    email_input: nwg::TextInput,
    
    password_label: nwg::Label,
    password_input: nwg::TextInput,
    
    login_button: nwg::Button,
    cancel_button: nwg::Button,
    
    result_sender: RefCell<Option<oneshot::Sender<Option<(String, String)>>>>,
}

#[cfg(windows)]
pub struct LoginDialogUi {
    inner: Rc<LoginDialog>,
    default_handler: RefCell<Option<nwg::EventHandler>>,
}

#[cfg(windows)]
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

#[cfg(windows)]
impl nwg::NativeUi<LoginDialogUi> for LoginDialog {
    fn build_ui(mut data: LoginDialog) -> Result<LoginDialogUi, nwg::NwgError> {
        // Controls
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .size((300, 150))
            .position((300, 300))
            .title("Miniover - Login")
            .build(&mut data.window)?;
            
        nwg::TextBox::builder()
            .text("Welcome to Miniover! Please enter your Pushover credentials to continue. Note: If you already have linked miniover as a client, you won't be able to login (remove the device from Pushover.net)")
            .parent(&data.window)
            .build(&mut data.text_box)?;

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

#[cfg(windows)]
impl Drop for LoginDialogUi {
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if let Some(handler) = handler.as_ref() {
            nwg::unbind_event_handler(handler);
        }
    }
}

#[cfg(windows)]
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

// ============================================================================
// Linux implementation using terminal stdin
// ============================================================================

#[cfg(target_os = "linux")]
use std::io::{self, Write};

#[cfg(target_os = "linux")]
pub async fn get_credentials() -> Option<(String, String)> {
    // Run the blocking stdin operations in a separate thread
    let result = tokio::task::spawn_blocking(|| {
        println!("\n=== Miniover Login ===");
        println!("Welcome to Miniover! Please enter your Pushover credentials.");
        println!("Note: If you already have linked miniover as a client, you won't be able to login");
        println!("(remove the device from Pushover.net first)\n");
        
        // Get email
        print!("Email: ");
        io::stdout().flush().ok()?;
        
        let mut email = String::new();
        io::stdin().read_line(&mut email).ok()?;
        let email = email.trim().to_string();
        
        if email.is_empty() {
            println!("Login cancelled.");
            return None;
        }
        
        // Get password securely (input is not echoed to terminal)
        print!("Password: ");
        io::stdout().flush().ok()?;
        
        let password = rpassword::read_password().ok()?;
        let password = password.trim().to_string();
        
        if password.is_empty() {
            println!("Login cancelled.");
            return None;
        }
        
        Some((email, password))
    }).await;
    
    result.unwrap_or(None)
}
