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
            .password(Some('*'))
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
                    nwg::Event::OnWindowClose => {
                        ui.cancel();
                    }
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
// Linux implementation using GTK4 with terminal fallback
// ============================================================================

#[cfg(target_os = "linux")]
use tokio::sync::oneshot;
#[cfg(target_os = "linux")]
use std::io::{self, Write, IsTerminal};
#[cfg(target_os = "linux")]
use std::sync::mpsc as std_mpsc;

#[cfg(target_os = "linux")]
use gtk4::prelude::*;
#[cfg(target_os = "linux")]
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, Entry, Label, Orientation, PasswordEntry};

#[cfg(target_os = "linux")]
pub async fn get_credentials() -> Option<(String, String)> {
    let (tx, rx) = oneshot::channel();
    
    // Try GTK GUI first, fall back to terminal if it fails
    std::thread::spawn(move || {
        if let Some(result) = try_gtk_dialog() {
            let _ = tx.send(Some(result));
        } else if let Some(result) = try_terminal_fallback() {
            let _ = tx.send(Some(result));
        } else {
            let _ = tx.send(None);
        }
    });
    
    rx.await.unwrap_or(None)
}

#[cfg(target_os = "linux")]
fn try_gtk_dialog() -> Option<(String, String)> {
    // Check if we have a display available
    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        return None;
    }

    let (result_tx, result_rx) = std_mpsc::channel::<Option<(String, String)>>();
    
    let app = Application::builder()
        .application_id("com.miniover.login")
        .build();
    
    let result_tx_clone = result_tx.clone();
    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Miniover - Login")
            .default_width(400)
            .default_height(250)
            .resizable(false)
            .build();
        
        // Main vertical container
        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);
        main_box.set_margin_start(20);
        main_box.set_margin_end(20);
        
        // Welcome message
        let welcome_label = Label::new(Some(
            "Welcome to Miniover!\n\nPlease enter your Pushover credentials.\n\
            Note: If you already have linked miniover as a client,\n\
            you won't be able to login (remove the device from Pushover.net first)"
        ));
        welcome_label.set_wrap(true);
        welcome_label.set_justify(gtk4::Justification::Center);
        main_box.append(&welcome_label);
        
        // Email row
        let email_box = GtkBox::new(Orientation::Horizontal, 8);
        let email_label = Label::new(Some("Email:"));
        email_label.set_width_chars(10);
        email_label.set_xalign(1.0);
        let email_entry = Entry::new();
        email_entry.set_hexpand(true);
        email_entry.set_placeholder_text(Some("your@email.com"));
        email_box.append(&email_label);
        email_box.append(&email_entry);
        main_box.append(&email_box);
        
        // Password row
        let password_box = GtkBox::new(Orientation::Horizontal, 8);
        let password_label = Label::new(Some("Password:"));
        password_label.set_width_chars(10);
        password_label.set_xalign(1.0);
        let password_entry = PasswordEntry::new();
        password_entry.set_hexpand(true);
        password_entry.set_show_peek_icon(true);
        password_box.append(&password_label);
        password_box.append(&password_entry);
        main_box.append(&password_box);
        
        // Button row
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(12);
        
        let cancel_button = Button::with_label("Cancel");
        let login_button = Button::with_label("Login");
        login_button.add_css_class("suggested-action");
        
        button_box.append(&cancel_button);
        button_box.append(&login_button);
        main_box.append(&button_box);
        
        window.set_child(Some(&main_box));
        
        // Clone for closures
        let result_tx_login = result_tx_clone.clone();
        let result_tx_cancel = result_tx_clone.clone();
        let result_tx_close = result_tx_clone.clone();
        
        let email_entry_clone = email_entry.clone();
        let password_entry_clone = password_entry.clone();
        let window_clone = window.clone();
        
        // Login button handler
        login_button.connect_clicked(move |_| {
            let email = email_entry_clone.text().to_string();
            let password = password_entry_clone.text().to_string();
            
            if !email.is_empty() && !password.is_empty() {
                let _ = result_tx_login.send(Some((email, password)));
            } else {
                let _ = result_tx_login.send(None);
            }
            window_clone.close();
        });
        
        // Cancel button handler
        let window_clone2 = window.clone();
        cancel_button.connect_clicked(move |_| {
            let _ = result_tx_cancel.send(None);
            window_clone2.close();
        });
        
        // Window close handler
        window.connect_close_request(move |_| {
            let _ = result_tx_close.send(None);
            gtk4::glib::Propagation::Proceed
        });
        
        window.present();
    });
    
    // Run the GTK application - this blocks until the window is closed
    let args: Vec<String> = vec![];
    app.run_with_args(&args);
    
    // Get the result
    result_rx.try_recv().ok().flatten()
}

#[cfg(target_os = "linux")]
fn try_terminal_fallback() -> Option<(String, String)> {
    // Check if we have a terminal for interactive input
    if !io::stdin().is_terminal() {
        eprintln!("No terminal or display available. Run `miniover` in a terminal or graphical session to configure credentials.");
        return None;
    }

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
}
