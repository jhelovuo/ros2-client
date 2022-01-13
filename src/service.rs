use std::io;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::message::Message;
use crate::pubsub::{Publisher,Subscription};
use crate::node::Node;

use rustdds::*;

use serde::{Serialize, Deserialize,};

#[derive(Clone,Copy,Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequenceNumber {
  number: i64,
}

impl SequenceNumber {
  pub fn new() -> SequenceNumber { 
    SequenceNumber{ number: 0 } 
  }

  pub fn from_high_low(high:i32, low:u32) -> SequenceNumber {
    SequenceNumber { 
      number: ((high as i64) << 32) + (low as i64)
    }
  }

  pub fn high(&self) -> i32 {
    (self.number >> 32) as i32
  }

  pub fn low(&self) -> u32 {
    (self.number & 0xFFFF_FFFF) as u32
  }

  pub fn next(&self) -> SequenceNumber {
    SequenceNumber{ number: self.number + 1 }
  }

}



/// [Original](https://docs.ros2.org/foxy/api/rmw/structrmw__request__id__t.html)
#[derive(Clone,Copy,Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmwRequestId {
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber, 
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
    type Response: Message;
    fn request_type_name() -> String;
    fn response_type_name() -> String;
}


pub struct Server<S:Service> {
  request_receiver: Subscription<RequestSerializationWrapper<S::Request>>,
  response_sender: Publisher<ResponseSerializationWrapper<S::Response>>,
}


impl<S: 'static + Service> Server<S> {
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Server<S>>
  {

    let request_receiver = node.create_subscription
      ::<RequestSerializationWrapper<S::Request>>(request_topic, qos.clone())?;
    let response_sender = node.create_publisher
      ::<ResponseSerializationWrapper<S::Response>>(response_topic, qos)?;
    info!("Created new Server: requests={} response={}", request_topic.name(), response_topic.name());

    Ok(Server { request_receiver, response_sender })
  }

  pub fn receive_request(&mut self) -> dds::Result<Option<(RmwRequestId,S::Request)>>
    where <S as Service>::Request: 'static
  {
    let rwo = self.request_receiver.take()?;
    Ok( rwo
        .map( |(rw, _message_info)| 
          ( RmwRequestId {
              writer_guid: rw.writer_guid,  
              sequence_number: 
                SequenceNumber::from_high_low(
                  rw.sequence_number_high, 
                  rw.sequence_number_low),
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
        sequence_number_high: id.sequence_number.high(),
        sequence_number_low: id.sequence_number.low(),
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
  request_sender: Publisher<RequestSerializationWrapper<S::Request>>,
  response_receiver: Subscription<ResponseSerializationWrapper<S::Response>>,
  sequence_number_counter: SequenceNumber,
}

impl<S: 'static + Service> Client<S> {
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Client<S>>
  {
    let request_sender = node.create_publisher
      ::<RequestSerializationWrapper<S::Request>>(request_topic, qos.clone())?;
    let response_receiver = node.create_subscription
      ::<ResponseSerializationWrapper<S::Response>>(response_topic, qos)?;
    info!("Created new Client: request topic={} response topic={}", request_topic.name(), response_topic.name());

    Ok( Client{ request_sender, response_receiver, 
                sequence_number_counter: SequenceNumber::new(), })
  }

  pub fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    let sn = self.sequence_number_counter;
    self.sequence_number_counter = self.sequence_number_counter.next();
    let writer_guid = self.request_sender.guid();

    self.request_sender.publish( 
      RequestSerializationWrapper {
        writer_guid,
        sequence_number_high: sn.high(),
        sequence_number_low: sn.low() ,
        instance_name: "".to_string(),
        request,
      }
    )?;

    Ok( RmwRequestId{writer_guid, sequence_number: sn} )
  }

  pub fn receive_response(&mut self) -> dds::Result<Option<(RmwRequestId,S::Response)>>
    where <S as Service>::Response: 'static
  {
    let rwo = self.response_receiver.take()?;
    Ok( rwo
        .map( |(rw, _message_info)| 
          ( RmwRequestId {
              writer_guid: rw.writer_guid,  
              sequence_number: SequenceNumber::from_high_low(
                rw.sequence_number_high, rw.sequence_number_low,
                ),
            },
            rw.response
          ) 
        )
      )
  }

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
