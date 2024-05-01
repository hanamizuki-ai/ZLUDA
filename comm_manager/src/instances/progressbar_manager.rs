use crate::util::encode_7bit_string;
use crate::{CommManagerInstance, CommManagerSwitch};
use std::io::Write;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::{self, JoinHandle};

pub struct ProgressBarManager {
    sender: Sender<ProgressCommand>,
    thread: JoinHandle<()>
}

pub struct ProgressBar<'a> {
    id: u64,
    manager: &'a ProgressBarManager,
}

impl<'a> ProgressBar<'a> {
    pub fn new(id: u64, manager: &'a ProgressBarManager) -> Self {
        Self { id, manager }
    }

    pub fn set_progress(&self, is_indeterminate: Option<bool>, max: Option<f64>, name: Option<String>) {
        self.manager.set_progress(self.id, is_indeterminate, max, name);
    }

    pub fn remove(&self) {
        self.manager.remove_progress(self.id);
    }

    pub fn set_left_text(&self, text: Option<String>) {
        self.manager.set_left_text(self.id, text);
    }

    pub fn set_right_text(&self, text: Option<String>) {
        self.manager.set_right_text(self.id, text);
    }

    pub fn set_progress_value(&self, value: f64) {
        self.manager.set_progress_value(self.id, value);
    }
}

impl<'a> Drop for ProgressBar<'a> {
    fn drop(&mut self) {
        self.remove();
    }
}

enum ProgressCommand {
    SetProgress(u64, bool, f64, Option<String>),
    RemoveProgress(u64),
    SetLeftText(u64, Option<String>),
    SetRightText(u64, Option<String>),
    SetProgressValue(u64, f64),
}

impl ProgressBarManager {
    pub fn new(pipe: CommManagerInstance) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = Self::start(pipe, receiver);
        Self { sender, thread }
    }

    pub fn from_switch(switch: &mut CommManagerSwitch) -> std::io::Result<Self> {
        let pipe = switch.acquire(b"\xff\x70\xc1\x2b\x1f\x44\x20\x46\xba\xab\x44\xa4\x70\xe3\xca\xb6")?;
        Ok(Self::new(pipe))
    }

    pub fn create_progress(&self, id: Option<u64>, is_indeterminate: Option<bool>, max: Option<f64>, name: Option<String>) -> ProgressBar {
        let id = id.unwrap_or_else(|| rand::random());
        let progress = ProgressBar::new(id, self);
        progress.set_progress(is_indeterminate, max, name);
        progress
    }

    pub fn set_progress(&self, id: u64, is_indeterminate: Option<bool>, max: Option<f64>, name: Option<String>) {
        let _ = self.sender.send(ProgressCommand::SetProgress(id, is_indeterminate.unwrap_or(false), max.unwrap_or(0.0), name));
    }

    pub fn remove_progress(&self, id: u64) {
        let _ = self.sender.send(ProgressCommand::RemoveProgress(id));
    }

    pub fn set_left_text(&self, id: u64, text: Option<String>) {
        let _ = self.sender.send(ProgressCommand::SetLeftText(id, text));
    }

    pub fn set_right_text(&self, id: u64, text: Option<String>) {
        let _ = self.sender.send(ProgressCommand::SetRightText(id, text));
    }

    pub fn set_progress_value(&self, id: u64, value: f64) {
        let _ = self.sender.send(ProgressCommand::SetProgressValue(id, value));
    }

    fn start(mut pipe: CommManagerInstance, receiver: Receiver<ProgressCommand>) -> JoinHandle<()> {
        thread::spawn(move || {
            for command in receiver {
                match command {
                    ProgressCommand::SetProgress(id, is_indeterminate, value, text) => {
                        // struct.pack('<HQ?d', 1, pid, indeterminate, maxp or 0) + encode_7bit_string(name)
                        // 2+8+1+8+string
                        let mut buffer = [0u8; 2 + 8 + 1 + 8];
                        buffer[0..2].copy_from_slice(&1u16.to_le_bytes());
                        buffer[2..10].copy_from_slice(&id.to_le_bytes());
                        buffer[10] = is_indeterminate as u8;
                        buffer[11..19].copy_from_slice(&value.to_le_bytes());
                        let _ = pipe.write_all(&buffer);
                        let _ = pipe.write_all(&encode_7bit_string(text.as_deref()));
                    }
                    ProgressCommand::RemoveProgress(id) => {
                        // struct.pack('<HQ', 2, pid)
                        let mut buffer = [0u8; 2 + 8];
                        buffer[0..2].copy_from_slice(&2u16.to_le_bytes());
                        buffer[2..10].copy_from_slice(&id.to_le_bytes());
                        let _ = pipe.write_all(&buffer);
                    }
                    ProgressCommand::SetLeftText(id, text) => {
                        // struct.pack('<HQ', 3, pid) + encode_7bit_string(text)
                        let mut buffer = [0u8; 2 + 8];
                        buffer[0..2].copy_from_slice(&3u16.to_le_bytes());
                        buffer[2..10].copy_from_slice(&id.to_le_bytes());
                        let _ = pipe.write_all(&buffer);
                        let _ = pipe.write_all(&encode_7bit_string(text.as_deref()));
                    }
                    ProgressCommand::SetRightText(id, text) => {
                        // struct.pack('<HQ', 4, pid) + encode_7bit_string(text)
                        let mut buffer = [0u8; 2 + 8];
                        buffer[0..2].copy_from_slice(&4u16.to_le_bytes());
                        buffer[2..10].copy_from_slice(&id.to_le_bytes());
                        let _ = pipe.write_all(&buffer);
                        let _ = pipe.write_all(&encode_7bit_string(text.as_deref()));
                    }
                    ProgressCommand::SetProgressValue(id, value) => {
                        // struct.pack('<HQd', 5, pid, value)
                        let mut buffer = [0u8; 2 + 8 + 8];
                        buffer[0..2].copy_from_slice(&5u16.to_le_bytes());
                        buffer[2..10].copy_from_slice(&id.to_le_bytes());
                        buffer[10..18].copy_from_slice(&value.to_le_bytes());
                        let _ = pipe.write_all(&buffer);
                    }
                }
            }
        })
    }
}

impl CommManagerSwitch {
    pub fn acquire_progressbar_manager(&mut self) -> std::io::Result<ProgressBarManager> {
        ProgressBarManager::from_switch(self)
    }
}
