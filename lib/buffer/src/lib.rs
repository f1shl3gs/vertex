mod channel;
mod config;
mod disk;
mod encoding;
pub mod queue;
mod receiver;
mod sender;

#[cfg(test)]
mod tests;

pub use channel::{LimitedReceiver, LimitedSender, limited};
pub use config::{BufferConfig, BufferType, WhenFull};
pub use encoding::Encodable;
pub use receiver::BufferReceiver;
pub use sender::BufferSender;

pub fn standalone_memory<T>(
    capacity: usize,
    when_full: WhenFull,
) -> (BufferSender<T>, BufferReceiver<T>)
where
    T: Encodable + Unpin,
{
    let (tx, rx) = channel::limited(capacity);
    let sender = BufferSender::memory(tx, when_full);
    let receiver = BufferReceiver::Memory(rx);
    (sender, receiver)
}
