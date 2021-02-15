//! Typed HTTP headers, pre-defined `HeaderName`s, traits for parsing and conversion, and other
//! header utility methods.

use std::fmt;

use bytes::{Bytes, BytesMut};
use percent_encoding::{AsciiSet, CONTROLS};

pub use http::header::*;

use crate::error::ParseError;
use crate::HttpMessage;

mod as_name;
mod into_pair;
mod into_value;
mod utils;

mod common;
pub(crate) mod map;
mod shared;

pub use self::common::*;
#[doc(hidden)]
pub use self::shared::*;

pub use self::as_name::AsHeaderName;
pub use self::into_pair::IntoHeaderPair;
pub use self::into_value::IntoHeaderValue;
#[doc(hidden)]
pub use self::map::GetAll;
pub use self::map::HeaderMap;
pub use self::utils::*;

/// A trait for any object that already represents a valid header field and value.
pub trait Header: IntoHeaderValue {
    /// Returns the name of the header field
    fn name() -> HeaderName;

    /// Parse a header
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError>;
}

#[derive(Debug, Default)]
pub(crate) struct Writer {
    buf: BytesMut,
}

impl Writer {
    fn new() -> Writer {
        Writer::default()
    }

    fn take(&mut self) -> Bytes {
        self.buf.split().freeze()
    }
}

impl fmt::Write for Writer {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        fmt::write(self, args)
    }
}

/// Convert `http::HeaderMap` to our `HeaderMap`.
impl From<http::HeaderMap> for HeaderMap {
    fn from(mut map: http::HeaderMap) -> HeaderMap {
        HeaderMap::from_drain(map.drain())
    }
}

/// This encode set is used for HTTP header values and is defined at
/// https://tools.ietf.org/html/rfc5987#section-3.2.
pub(crate) const HTTP_VALUE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'%')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'-')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'{')
    .add(b'}');
