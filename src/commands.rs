mod create;
mod launch;
mod instance;

pub use create::*;
pub use launch::*;

pub trait Progress {
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);
    fn advance(&mut self, current: usize);
}
