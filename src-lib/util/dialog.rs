use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread::JoinHandle;

use lazy_static::lazy_static;
use native_dialog::{FileDialog, MessageDialog, MessageType};

lazy_static! {

    // this is some arcane bullshit to get a global mpsc
    // the sender can be cloned, and we'll do that via a thread_local later
    // the receiver can't be cloned, so just shove it in an Option so we can take() it later.
    static ref DIALOG_REQUEST_CHANNEL: (Mutex<mpsc::Sender<DialogRequest>>, Mutex<Option<mpsc::Receiver<DialogRequest>>>) = {
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        let receiver = Mutex::new(Some(receiver));
        (sender, receiver)
    };
}

thread_local! {
    // We only need one of these per thread. As we don't use any thread pools this should be a one-time cost on application startup.
    static DIALOG_REQUEST_SENDER: mpsc::Sender<DialogRequest> = DIALOG_REQUEST_CHANNEL.0.lock().unwrap().clone();
}

/// The different types of requests the dialog worker thread can process
enum DialogRequest {
    /// Show a file browser for the user to select a PNG image
    PngPath,
    /// Show an informational popup with the provided text
    Info(String),
    /// Show a warning popup with the provided text
    Warning(String),
    /// Stop the dialog worker thread
    Terminate,
}

pub struct DialogWorker {
    join_handle: Option<JoinHandle<()>>,
    file_path_receiver: mpsc::Receiver<Option<PathBuf>>,
}

impl DialogWorker {
    /// try to get a file path from the dialog worker's internal queue
    pub fn try_recv_file_path(&self) -> Result<Option<PathBuf>, mpsc::TryRecvError> {
        self.file_path_receiver.try_recv()
    }

    /// signal the dialog worker thread to shut down once it's done processing its queue
    pub fn shutdown(&mut self) -> Option<()> {
        let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Terminate));
        self.join_handle.take()?.join().ok()
    }

}

/// show a native popup with an info icon + sound
pub fn show_info(text: String) {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Info(text)));
}

/// show a native popup with a warning icon + sound
pub fn show_warning(text: String) {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::Warning(text)));
}

/// show a native popup requesting a path to a PNG
pub fn request_png() {
    let _ = DIALOG_REQUEST_SENDER.with(|sender| sender.send(DialogRequest::PngPath));
}

pub fn spawn_worker() -> DialogWorker {
    let (file_path_sender, file_path_receiver) = mpsc::channel();
    let dialog_request_receiver = DIALOG_REQUEST_CHANNEL.1.lock().unwrap().take().unwrap();

    // native dialogs block a thread, so we'll spin up a single thread to loop through queued dialogs.
    // If we ever need to show multiple dialogs, they just get queued.
    let join_handle = std::thread::Builder::new()
        .name("dialog-worker".to_string())
        .spawn(move || {
            loop {
                // block waiting for a file read request
                match dialog_request_receiver.recv().unwrap() {
                    DialogRequest::PngPath => {
                        let path = FileDialog::new()
                            .add_filter("PNG Image", &["png"])
                            .show_open_single_file()
                            .ok()
                            .flatten();

                        let _ = file_path_sender.send(path);
                    }
                    DialogRequest::Info(text) => {
                        MessageDialog::new()
                            .set_type(MessageType::Info)
                            .set_title("Simple Crosshair Overlay")
                            .set_text(&text)
                            .show_alert()
                            .unwrap();
                    }
                    DialogRequest::Warning(text) => {
                        MessageDialog::new()
                            .set_type(MessageType::Warning)
                            .set_title("Simple Crosshair Overlay")
                            .set_text(&text)
                            .show_alert()
                            .unwrap();
                    }
                    DialogRequest::Terminate => break,
                }
            }
        }).unwrap();

    DialogWorker {
        join_handle: Some(join_handle), // we take() from this later
        file_path_receiver,
    }
}
