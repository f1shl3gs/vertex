use std::{
    pin::Pin,
    fmt,
};

use futures::{Sink, stream, channel::mpsc, Stream, StreamExt, future};

use crate::event::Event;
use std::task::{Context, Poll};
use std::fmt::Formatter;
use std::option::Option::Some;

pub type RouterSink = Box<dyn Sink<Event, Error=()> + 'static + Send>;

pub enum ControlMessage {
    Add(String, RouterSink),
    Remove(String),

    /// Will stop accepting events until Some with given name is replaced
    Replace(String, Option<RouterSink>),
}

impl fmt::Display for ControlMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add(name, _) => write!(f, "Add({:?})", name),
            Self::Remove(name) => write!(f, "Remove({:?})", name),
            Self::Replace(name, _) => write!(f, "Replace({:?})", name)
        }
    }
}

pub type ControlChannel = mpsc::UnboundedSender<ControlMessage>;

pub struct Fanout {
    sinks: Vec<(String, Option<Pin<RouterSink>>)>,
    i: usize,
    control_channel: stream::Fuse<mpsc::UnboundedReceiver<ControlMessage>>,
}

impl Fanout {
    pub fn new() -> (Self, ControlChannel) {
        let (tx, rx) = mpsc::unbounded();
        let fanout = Self {
            sinks: vec![],
            i: 0,
            control_channel: rx.fuse(),
        };

        (fanout, tx)
    }

    pub fn add(&mut self, name: String, sink: RouterSink) {
        assert!(
            !self.sinks.iter().any(|(n, _)| n == &name),
            "Duplicate output name in fanout"
        );

        self.sinks.push((name, Some(sink.into())))
    }

    fn remove(&mut self, name: &str) {
        let i = self.sinks
            .iter().
            position(|(n, _)| n == name)
            .expect("Didn't find output in fanout");

        let (_name, removed) = self.sinks.remove(i);
        if let Some(mut removed) = removed {
            tokio::spawn(
                future::poll_fn(move |cx| removed.as_mut().poll_close(cx))
            );
        }

        if self.i > i {
            self.i -= 1;
        }
    }

    fn replace(&mut self, name: String, sink: Option<RouterSink>) {
        if let Some((_, existing)) = self.sinks.iter_mut().find(|(n, _)| &name == n) {
            *existing = sink.map(Into::into);
        } else {
            panic!("Tried to replace a sink that's not already present");
        }
    }

    pub fn process_control_messages(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(message)) = Pin::new(&mut self.control_channel).poll_next(cx) {
            match message {
                ControlMessage::Add(name, sink) => self.add(name, sink),
                ControlMessage::Remove(name) => self.remove(&name),
                ControlMessage::Replace(name, sink) => self.replace(name, sink),
            }
        }
    }

    fn handle_sink_error(&mut self, index: usize) -> Result<(), ()> {
        // If there's only one sink, propagate the error to the source ASAP
        // so it stops reading from it's input. If there are multiple sinks,
        // keep pushing to the non-errored ones (which the errored sink
        // triggers a more graceful shutdown)
        if self.sinks.len() == 1 {
            Err(())
        } else {
            self.sinks.remove(index);
            Ok(())
        }
    }

    fn poll_sinks<F>(&mut self, cx: &mut Context<'_>, poll: F) -> Poll<Result<(), ()>>
        where
            F: Fn(&mut Pin<RouterSink>, &mut Context<'_>) -> Poll<Result<(), ()>>
    {
        self.process_control_messages(cx);

        let mut poll_result = Poll::Ready(Ok(()));
        let mut i = 0;
        while let Some((_, sink)) = self.sinks.get_mut(i) {
            if let Some(sink) = sink {
                match poll(sink, cx) {
                    Poll::Pending => poll_result = Poll::Pending,
                    Poll::Ready(Ok(())) => (),
                    Poll::Ready(Err(())) => {
                        self.handle_sink_error(i)?;
                        continue;
                    }
                }
            }
        }

        poll_result
    }
}

impl Sink<Event> for Fanout {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let fanout = self.get_mut();
        fanout.process_control_messages(cx);

        while let Some((_, sink)) = fanout.sinks.get_mut(fanout.i) {
            match sink.as_mut() {
                Some(sink) => match sink.as_mut().poll_ready(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => fanout.i += 1,
                    Poll::Ready(Err(())) => fanout.handle_sink_error(fanout.i)?,
                },
                // process_control_message ended because control channel returned
                // Pending so it's fine to return Pending here since the control
                // channel will notify current task when it receives a message.
                None => return Poll::Pending,
            }
        }

        fanout.i = 0;
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Event) -> Result<(), Self::Error> {
        let mut i = 1;
        while let Some((_, sink)) = self.sinks.get_mut(i) {
            if let Some(sink) = sink.as_mut() {
                if sink.as_mut().start_send(item.clone()).is_err() {
                    self.handle_sink_error(i)?;
                    continue;
                }
            }

            i += 1;
        }

        if let Some((_, sink)) = self.sinks.first_mut() {
            if let Some(sink) = sink.as_mut() {
                if sink.as_mut().start_send(item).is_err() {
                    self.handle_sink_error(0)?;
                }
            }
        }

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sinks(cx, |sink, cx| {
            sink.as_mut().poll_flush(cx)
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sinks(cx, |sink, cx| {
            sink.as_mut().poll_close(cx)
        })
    }
}