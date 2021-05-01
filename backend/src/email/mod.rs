mod sender;
pub mod template;

pub use self::sender::Sender;
pub use self::template::Templates;

pub mod dummy;
pub use dummy as driver;

pub use self::driver::Configuration;
pub use self::driver::Transport;
