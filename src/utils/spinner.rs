use crate::utils::logger::{LogLevel, Logger};
use indicatif::{ProgressBar, ProgressStyle};
use std::cell::Cell;
use std::time::Duration;

pub struct Spinner {
    bar: ProgressBar,
    active: Cell<bool>,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        let bar = ProgressBar::new_spinner();
        let style = ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        bar.set_style(style);
        bar.set_message(message.into());
        bar.enable_steady_tick(Duration::from_millis(80));

        Spinner {
            bar,
            active: Cell::new(true),
        }
    }

    #[allow(dead_code)]
    pub fn set_message(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }
    #[allow(dead_code)]
    pub fn set_message_allow_dead(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }

    #[allow(dead_code)]
    fn _set_message_allow_dead(&self, _message: impl Into<String>) {
        // kept for API compatibility in other builds; no-op when unused
    }

    pub fn succeed(&self, message: impl Into<String>) {
        if self.active.get() {
            // Clear spinner then emit a structured success log via Logger
            self.bar.finish_and_clear();
            Logger::new().log_message(LogLevel::Success, &message.into().to_string());
            self.active.set(false);
        }
    }

    pub fn fail(&self, message: impl Into<String>) {
        if self.active.get() {
            // Clear spinner then emit a structured error log via Logger
            self.bar.finish_and_clear();
            Logger::new().log_message(LogLevel::Error, &message.into().to_string());
            self.active.set(false);
        }
    }

    pub fn finish_and_clear(&self) {
        if self.active.get() {
            self.bar.finish_and_clear();
            self.active.set(false);
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.active.get() {
            self.bar.abandon();
            // Note: active is Cell but we have &mut self here, so set directly
            self.active.set(false);
        }
    }
}

pub fn with_spinner(message: &str) -> Spinner {
    Spinner::new(message)
}

pub fn run_step<T, F, S>(start_message: &str, on_success: S, action: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
    S: FnOnce(&T) -> String,
{
    let spinner = Spinner::new(start_message);
    match action() {
        Ok(value) => {
            let message = on_success(&value);
            spinner.succeed(message);
            Ok(value)
        }
        Err(err) => {
            spinner.fail(err.clone());
            Err(err)
        }
    }
}

pub fn run_unit_step<F>(start_message: &str, success_message: &str, action: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    run_step(start_message, |_| success_message.to_string(), action)
}
