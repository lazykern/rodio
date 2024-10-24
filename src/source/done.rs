use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::sync::Mutex;
use derivative::Derivative;

use crate::{Sample, Source};

use super::SeekError;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Done<I> {
    input: I,
    signal: Arc<AtomicUsize>,
    signal_sent: bool,
    #[derivative(Debug="ignore")]
    on_done: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
}

impl<I> Done<I> {
    /// When the inner source is empty the AtomicUsize passed in is decremented.
    /// If it was zero it will overflow negatively.
    #[inline]
    pub fn new(input: I, signal: Arc<AtomicUsize>) -> Done<I> {
        Done {
            input,
            signal,
            signal_sent: false,
            on_done: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input
    }

    // Add method to set callback
    #[inline]
    pub fn set_on_done<F>(&self, callback: F)
    where
        F: Fn() + Send + 'static,
    {
        *self.on_done.lock().unwrap() = Some(Box::new(callback));
    }
}

impl<I: Source> Iterator for Done<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let next = self.input.next();
        if !self.signal_sent && next.is_none() {
            self.signal.fetch_sub(1, Ordering::Relaxed);
            self.signal_sent = true;
            // Execute callback when song ends
            if let Some(callback) = &*self.on_done.lock().unwrap() {
                callback();
            }
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for Done<I>
where
    I: Source,
    I::Item: Sample,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)
    }
}
