mod auth;
mod create;
mod launch;

pub use auth::*;
pub use create::*;
pub use launch::*;

mod account;
mod instance;

pub trait Progress {
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);
    fn advance(&mut self, current: usize);
}
