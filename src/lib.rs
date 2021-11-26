//! Some simple utilities for hand-coded parsers.
use std::fmt;
use std::slice::SliceIndex;

use thiserror::Error;

/// A trait for error types with a variant that indicates that the end of the parsed input has been
/// reached unexpectedly.
pub trait Eoi {
    /// Create the instance of the error type thatnotes an unexpected end of input.
    fn eoi() -> Self;
}

/// Wraps a slice of input bytes to provide methods for advancing through the input, tracking
/// position, signaling parse errors, looking ahead, etc.
pub struct ParserHelper<'a> {
    input: &'a [u8],
    position: usize,
}

/// A parse error, tagging an arbitrary error type with an input position.
#[derive(Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[error("parse error at position {position}: {e}")]
pub struct Error<E> {
    pub position: usize,
    pub e: E,
}

impl<E> Error<E> {
    /// Create a new error. You should probably use `ParserHelper` methods instead.
    pub fn new(position: usize, e: E) -> Self {
        Error {
            position,
            e,
        }
    }
}

impl<E: serde::de::Error> serde::de::Error for Error<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(0, E::custom(msg))
    }
}

impl<'a> ParserHelper<'a> {
    /// Parses from a slice of bytes.
    pub fn new(input: &'a [u8]) -> Self {
        ParserHelper {
            input,
            position: 0,
        }
    }

    /// Return the total length of the input.
    pub fn len(&self) -> usize {
        self.input.len()
    }

    /// Obtain a slice into the original input.
    pub fn slice<I: SliceIndex<[u8]>>(&self, i: I) -> &'a I::Output {
        &self.input[i]
    }

    /// Reference to portion of buffer yet to be parsed
    pub fn rest(&self) -> &'a [u8] {
        self.slice(self.position()..)
    }

    /// Current byte offset of buffer being parsed
    pub fn position(&self) -> usize {
        self.position
    }

    /// Produce an error at the current position.
    pub fn fail<T, E>(&self, reason: E) -> Result<T, Error<E>> {
        self.fail_at_position(reason, self.position())
    }

    /// Produce an error at the given position.
    pub fn fail_at_position<T, E>(&self, reason: E, position: usize) -> Result<T, Error<E>> {
        Err(Error::new(position, reason))
    }

    /// Produce an error indicating the unexpected end of the input at the current position.
    pub fn unexpected_end_of_input<T, E: Eoi>(&self) -> Result<T, Error<E>> {
        self.fail(E::eoi())
    }

    /// Advance the input slice by some number of bytes.
    pub fn advance(&mut self, offset: usize) {
        self.position += offset;
    }

    /// Advance the input but only if it matches the given bytes, returns whether it did advance.
    pub fn advance_over(&mut self, expected: &[u8]) -> bool {
        if self.rest().starts_with(expected) {
            self.advance(expected.len());
            return true;
        } else {
            return false;
        }
    }

    /// Advance the input slice by some number of bytes, returning the given error if not enough
    /// input is available.
    pub fn advance_or<E>(&mut self, offset: usize, e: E) -> Result<(), Error<E>> {
        let start = self.position;
        self.position += offset;
        if self.len() < self.position {
            return self.fail_at_position(e, start);
        } else {
            return Ok(());
        }
    }

    /// Consumes the next byte and returns it.
    /// Signals unexpected end of the input if no next byte is available.
    pub fn next<E: Eoi>(&mut self) -> Result<u8, Error<E>> {
        if let Some(c) = self.input.get(self.position()) {
            self.advance(1);
            Ok(*c)
        } else {
            self.unexpected_end_of_input()
        }
    }

    /// Consumes the next byte and returns it, or signals end of input as `None`.
    pub fn next_or_end(&mut self) -> Option<u8> {
        if let Some(c) = self.input.get(self.position()) {
            self.advance(1);
            Some(*c)
        } else {
            None
        }
    }

    /// Consumes the expected byte, gives the given error if it is something else.
    pub fn expect<E: Eoi>(&mut self, expected: u8, err: E) -> Result<(), Error<E>> {
        let pos = self.position();
        if self.next()? == expected {
            Ok(())
        } else {
            self.fail_at_position(err, pos)
        }
    }

    /// Same as `expect`, but for multiple consecutive bytes.
    pub fn expect_bytes<E>(&mut self, exp: &[u8], err: E) -> Result<(), Error<E>> {
        if self.rest().starts_with(exp) {
            self.advance(exp.len());
            Ok(())
        } else {
            self.fail(err)
        }
    }

    /// Same as expect, but using a predicate.
    pub fn expect_pred<E: Eoi>(&mut self, pred: fn(u8) -> bool, err: E) -> Result<(), Error<E>> {
        let pos = self.position();
        if pred(self.next()?) {
            Ok(())
        } else {
            self.fail_at_position(err, pos)
        }
    }

    /// Returns the next byte without consuming it.
    /// Signals unexpected end of the input if no next byte is available.
    pub fn peek<E: Eoi>(&self) -> Result<u8, Error<E>> {
        if let Some(c) = self.input.get(self.position()) {
            Ok(*c)
        } else {
            self.unexpected_end_of_input()
        }
    }

    /// Returns the next byte without consuming it, or signals end of input as `None`.
    pub fn peek_or_end(&self) -> Option<u8> {
        self.input.get(self.position()).copied()
    }

    /// Skips values while the predicate returns true.
    pub fn skip(&mut self, pred: fn(u8) -> bool) {
        loop {
            match self.peek_or_end() {
                None => return,
                Some(peeked) => {
                    if pred(peeked) {
                        self.advance(1);
                    } else {
                        return;
                    }
                }
            }
        }
    }
}
