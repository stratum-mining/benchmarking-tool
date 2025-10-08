mod into_static;
mod message_channel;
mod proxy_builder;

pub use into_static::into_static;
pub use message_channel::{MessageChannel, Remote};
pub use proxy_builder::ProxyBuilder;

use codec_sv2::{StandardEitherFrame, StandardSv2Frame};
use stratum_common::roles_logic_sv2::parsers_sv2::AnyMessage;

pub type Frame_ = StandardEitherFrame<AnyMessage<'static>>;
pub type StdFrame = StandardSv2Frame<AnyMessage<'static>>;
