use std::io;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

use crate::message::Message;
use crate::pubsub::{Publisher,Subscription};

pub trait Service {
    type Request: Message;
    type Response: Message;
}


pub struct Server<S:Service> {
  request_receiver: Subscription<S::Request>,
  response_sender: Publisher<S::Response>,
}

impl<S:Service> Evented for Server<S> {
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self
      .request_receiver
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self
      .request_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.request_receiver.deregister(poll)
  }

}

pub struct Client<S:Service> {
  request_sender: Publisher<S::Request>,
  response_receiver: Subscription<S::Response>,
}

impl<S:Service> Evented for Client<S> {
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self
      .response_receiver
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self
      .response_receiver
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.response_receiver.deregister(poll)
  }

}
