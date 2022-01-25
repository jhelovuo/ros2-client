use std::io;
use std::convert::TryFrom;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::message::Message;
use crate::pubsub::{Publisher,Subscription, MessageInfo, };
use crate::node::Node;

use rustdds::*;
use rustdds::rpc::*;

use serde::{Serialize, Deserialize,};

use concat_arrays::concat_arrays;

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

impl From<i64> for SequenceNumber {
  fn from(number:i64) -> SequenceNumber {
    SequenceNumber{ number }
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

// --------------------------------------------
// --------------------------------------------

// See Spec RPC over DDS Section "7.2.4 Basic and Enhanced Service Mapping for RPC over DDS"
pub trait ServiceWrapper<R> {
  type State;
  fn wrap(state: &mut Self::State, r_id: RmwRequestId, response_or_request:R) -> Self;
  fn request_id_after_wrap(state: &mut Self::State, write_result:SampleIdentity) -> RmwRequestId;
  fn unwrap(state: &mut Self::State, wrapped: Self, sample_info: SampleInfo) -> (RmwRequestId, R);
}

// --------------------------------------------
// --------------------------------------------

// This is reverse-engineered from
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/rmw_node.cpp
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/serdata.hpp
#[derive(Serialize,Deserialize)]
struct CycloneWrapper<R> {
  guid_second_half: [u8;8], // CycolenDDS RMW only sends last 8 bytes of client GUID
  sequence_number_high: i32,
  sequence_number_low: u32,
  response_or_request: R,  // ROS2 payload  
}

fn cyclone_wrap<R>(r_id: RmwRequestId, response_or_request:R ) -> CycloneWrapper<R> {
  let sn = r_id.sequence_number;

  let mut guid_second_half = [0;8];
  // writer_guid means client GUID (i.e. request writer)
  guid_second_half.copy_from_slice( &r_id.writer_guid.to_bytes()[8..16] );

  CycloneWrapper{
    guid_second_half,
    sequence_number_high: sn.high(),
    sequence_number_low: sn.low(),
    response_or_request,
  }
}

fn cyclone_unwrap<R>(wrapped: CycloneWrapper<R> , metadata: MessageInfo, 
                      my_guid_if_client:Option<GUID> ) -> (RmwRequestId, R) 
{
  let mut first_half = [0;8];
 
  match my_guid_if_client {
    Some(client_guid) => 
      first_half.copy_from_slice( &client_guid.to_bytes().as_slice()[0..8]), 
      // this seems a bit odd, but source is
      // https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_impl.cpp
      // function take_response()
    None => 
      first_half.copy_from_slice(&metadata.writer_guid.to_bytes().as_slice()[0..8]), 
      // we are server, so writer is client
  }

  // This is received in the wrapper header
  let mut second_half = [0;8];
  second_half.copy_from_slice( &metadata.writer_guid.to_bytes()[8..16] );

  let r_id = RmwRequestId {
    writer_guid: GUID::from_bytes(concat_arrays!(first_half,second_half)),
    sequence_number: SequenceNumber::
      from_high_low(wrapped.sequence_number_high, wrapped.sequence_number_low),
  };

  ( r_id, wrapped.response_or_request )
} 

// --------------------------------------------
// --------------------------------------------

pub trait Service {
    type Request: Message;
    type Response: Message;
    fn request_type_name() -> String;
    fn response_type_name() -> String;
}


pub struct Server<S:Service> {
  request_receiver: Subscription<CycloneWrapper<S::Request>>,
  response_sender: Publisher<CycloneWrapper<S::Response>>,
}


impl<S: 'static + Service> Server<S> {
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Server<S>>
  {

    let request_receiver = node.create_subscription
      ::<CycloneWrapper<S::Request>>(request_topic, qos.clone())?;
    let response_sender = node.create_publisher
      ::<CycloneWrapper<S::Response>>(response_topic, qos)?;
    info!("Created new Server: requests={} response={}", request_topic.name(), response_topic.name());

    Ok(Server { request_receiver, response_sender })
  }

  pub fn receive_request(&mut self) -> dds::Result<Option<(RmwRequestId,S::Request)>>
    where <S as Service>::Request: 'static
  {
    let rwo = self.request_receiver.take()?;
    Ok( rwo.map( |(rw, message_info)| cyclone_unwrap(rw, message_info, None) )
      )
  }

  pub fn send_response(&self, id:RmwRequestId, response: S::Response) -> dds::Result<()> {
    self.response_sender.publish( cyclone_wrap(id, response))
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
  request_sender: Publisher<CycloneWrapper<S::Request>>,
  response_receiver: Subscription<CycloneWrapper<S::Response>>,
  sequence_number_counter: SequenceNumber,
}

impl<S: 'static + Service> Client<S> {
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Client<S>>
  {
    let request_sender = node.create_publisher
      ::<CycloneWrapper<S::Request>>(request_topic, qos.clone())?;
    let response_receiver = node.create_subscription
      ::<CycloneWrapper<S::Response>>(response_topic, qos)?;
    info!("Created new Client: request topic={} response topic={}", request_topic.name(), response_topic.name());

    Ok( Client{ request_sender, response_receiver, 
                sequence_number_counter: SequenceNumber::new(), })
  }

  pub fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    let sequence_number = self.sequence_number_counter;
    self.sequence_number_counter = self.sequence_number_counter.next();

    // Generate new request id
    let request_id = RmwRequestId {
      writer_guid: self.request_sender.guid(),
      sequence_number,
    };

    self.request_sender.publish( cyclone_wrap(request_id, request) )?;

    Ok( request_id )
  }

  pub fn receive_response(&mut self) -> dds::Result<Option<(RmwRequestId,S::Response)>>
    where <S as Service>::Response: 'static
  {
    let rwo = self.response_receiver.take()?;
    Ok( rwo.map( |(rw, message_info)| 
                    cyclone_unwrap(rw, message_info, Some(self.request_sender.guid()))))
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
