use std::io;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

use crate::message::Message;
use crate::pubsub::{Publisher,Subscription};

use rustdds::*;

use serde::{Serialize, Deserialize,};

/// [Original](https://docs.ros2.org/foxy/api/rmw/structrmw__request__id__t.html)
pub struct RmwRequestId {
  pub writer_guid: GUID,
  pub sequence_number: i64, 
}

/// [original](https://docs.ros2.org/foxy/api/rmw/structrmw__service__info__t.html)
// But where is this used?
//
// pub struct RmwServiceInfo {
//   pub source_timestamp: RmwTimePointValue,
//   pub received_timestamp: RmwTimePointValue,
//   pub request_id: RmwRequestId,
// }

// This is reverse-engineered from
// https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_type_support.cpp
// * RMW_Connext_RequestReplyMapping_Basic_serialize
// * RMW_Connext_MessageTypeSupport::serialize
#[derive(Serialize,Deserialize)]
struct RequestSerializationWrapper<R> {
  writer_guid: GUID,
  sequence_number_high: i32,
  sequence_number_low: u32,
  instance_name: String, // apparently, this is always sent as the empty string
  request: R,
}

#[derive(Serialize,Deserialize)]
struct ResponseSerializationWrapper<R> {
  writer_guid: GUID,
  sequence_number_high: i32,
  sequence_number_low: u32,
  sample_rc: u32, // apparently, this is always sent as 0. But what is it?
  response: R,
}


pub trait Service {
    type Request: Message;
    type Response:Message;
}


pub struct Server<S:Service> {
  request_receiver: Subscription<RequestSerializationWrapper<S::Request>>,
  response_sender: Publisher<ResponseSerializationWrapper<S::Response>>,
}


impl<S:Service> Server<S> {
  pub fn new()
  {}

  pub fn receive_request(&mut self) -> dds::Result<Option<(RmwRequestId,S::Request)>>
    where <S as Service>::Request: 'static
  {
    let rwo = self.request_receiver.take()?;
    Ok( rwo
        .map( |(rw, _message_info)| 
          ( RmwRequestId {
              writer_guid: rw.writer_guid,  
              sequence_number: ((rw.sequence_number_high as i64) << 32) 
                + (rw.sequence_number_low as i64),
            },
            rw.request
          ) 
        )
      )
  }

  pub fn send_response(&self, id:RmwRequestId, response: S::Response) -> dds::Result<()> {
    self.response_sender.publish( 
      ResponseSerializationWrapper {
        writer_guid: id.writer_guid,
        sequence_number_high: (id.sequence_number >> 32) as i32,
        sequence_number_low: (id.sequence_number & 0xFFFF_FFFF) as u32,
        sample_rc: 0,
        response,
      }
    )
  }
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

// -------------------------------------------------------------------
// -------------------------------------------------------------------

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
