use std::marker::PhantomData;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::message::Message;
use crate::node::Node;
use crate::pubsub::{Publisher, Subscription, MessageInfo};

use rustdds::*;
use rustdds::rpc::*;

use serde::{Serialize, Deserialize,};

//use concat_arrays::concat_arrays;

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
pub trait ServiceServerWrapper<Q,P> 
where 
  Self::RequestWrapper: Message,
  Self::ResponseWrapper: Message,
{
  type RequestWrapper;
  type ResponseWrapper;
  // Unwrapping will clone the request
  // This is reasonable, because we may have to take it out of another struct
  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, Q);

  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>);
}

//pub trait ServiceClientWrapper<Q,P> {
//fn wrap_request(state: &mut Self::State, request:R) -> Self;
// type State;
//  fn request_id_after_wrap(state: &mut Self::State, write_result:SampleIdentity) -> RmwRequestId;
//  fn unwrap_response(state: &mut Self::State, wrapped: Self, sample_info: SampleInfo) -> (RmwRequestId, R);
//  }
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

impl<R:Message> Message for CycloneWrapper<R> {}

struct CycloneServerWrapper<Q,P> 
{
  request_phantom: PhantomData<Q>,
  response_phantom: PhantomData<P>,
}


// struct CycloneWrapperState {
//  client_guid: GUID,
//  sequence_number_counter: SequenceNumber,
// }

impl<Q,P> ServiceServerWrapper<Q,P> for CycloneServerWrapper<Q,P> 
where
  Q: Message + Clone,
  P: Message,
{

  type RequestWrapper = CycloneWrapper<Q>;
  type ResponseWrapper = CycloneWrapper<P>;

  // fn wrap_request(state: &mut Self::State, request:R) -> Self {
  //   state.sequence_number_counter = state.sequence_number_counter.next();
  //   let sequence_number = state.sequence_number_counter;
    

  //   // Generate new request id
  //   let request_id = RmwRequestId {
  //     writer_guid: state.client_guid,
  //     sequence_number,
  //   };

  //   cyclone_wrap(request_id, request)
  // }

  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    (cyclone_wrap(r_id,response), None)
  }

  // fn request_id_after_wrap(state: &mut Self::State, _write_result:SampleIdentity) -> RmwRequestId {
  //   // request id is what we generated into header. write_result is irrelevant
  //   RmwRequestId {
  //     writer_guid: state.client_guid,
  //     sequence_number: state.sequence_number_counter,
  //   }
  // }

  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, Q) {
    let r_id = RmwRequestId {
      writer_guid: sample_info.writer_guid(),
      // Last 8 bytes of writer (client) GUID should be in the wrapper also
      sequence_number: SequenceNumber::
        from_high_low(wrapped.sequence_number_high, wrapped.sequence_number_low),
    };

    ( r_id, wrapped.response_or_request.clone() )
  }

  // fn unwrap_response(state: &mut Self::State, wrapped: Self, sample_info: SampleInfo) -> (RmwRequestId, R) {
  //   let mut first_half = [0;8];
  //   first_half.copy_from_slice( &state.client_guid.to_bytes().as_slice()[0..8]);
  //   // this seems a bit odd, but source is
  //   // https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_impl.cpp
  //   // function take_response()

  //   // This is received in the wrapper header
  //   let mut second_half = [0;8];
  //   second_half.copy_from_slice( &sample_info.writer_guid.to_bytes()[8..16] );

  //   let r_id = RmwRequestId {
  //     writer_guid: GUID::from_bytes(concat_arrays!(first_half,second_half)),
  //     sequence_number: SequenceNumber::
  //       from_high_low(wrapped.sequence_number_high, wrapped.sequence_number_low),
  //   };

  //   ( r_id, wrapped.response_or_request )
  // }
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

// --------------------------------------------
// --------------------------------------------

pub trait Service {
    type Request: Message;
    type Response: Message;
    fn request_type_name() -> String;
    fn response_type_name() -> String;
}


pub struct Server<S:Service, W:ServiceServerWrapper<S::Request,S::Response>>
{
  request_receiver: Subscription<W::RequestWrapper>,
  response_sender: Publisher<W::ResponseWrapper>,
  phantom: PhantomData<S>,
}


impl<S,W> Server<S,W>
where
  S: 'static + Service,
  W: 'static + ServiceServerWrapper<S::Request, S::Response>,
{
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) 
    -> dds::Result<Server<S,W>>
  {

    let request_receiver = node
      .create_subscription::<W::RequestWrapper>(request_topic, qos.clone())?;
    let response_sender = node
      .create_publisher::<W::ResponseWrapper>(response_topic, qos)?;

    info!("Created new Server: requests={} response={}", request_topic.name(), response_topic.name());

    Ok(Server { request_receiver, response_sender, phantom:PhantomData })
  }

  pub fn receive_request(&mut self) -> dds::Result<Option<(RmwRequestId,S::Request)>>
    where <S as Service>::Request: 'static
  {
    let next_sample = self.request_receiver.take()?;

    Ok( next_sample.map( |(s,mi)| W::unwrap_request(&s, &mi.sample_info ) ) )
  }

  pub fn send_response(&self, id:RmwRequestId, response: S::Response) -> dds::Result<()> {
    let (wrapped_response, rsi_opt) = W::wrap_response(id, response);
    let write_opt = WriteOptionsBuilder::new().related_sample_identity_opt(rsi_opt);
    self.response_sender.publish_with_options(wrapped_response, write_opt.build() )?;
    Ok(())
  }
}


// impl<S:Service, W:ServiceServerWrapper> Evented for Server<S,W> {
//   fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
//     self
//       .request_receiver
//       .register(poll, token, interest, opts)
//   }

//   fn reregister(
//     &self,
//     poll: &Poll,
//     token: Token,
//     interest: Ready,
//     opts: PollOpt,
//   ) -> io::Result<()> {
//     self
//       .request_receiver
//       .reregister(poll, token, interest, opts)
//   }

//   fn deregister(&self, poll: &Poll) -> io::Result<()> {
//     self.request_receiver.deregister(poll)
//   }

// }

// -------------------------------------------------------------------
// -------------------------------------------------------------------

pub struct Client<S:Service> {
  request_sender: no_key::DataWriterCdr<S::Request>,
  response_receiver: no_key::DataReaderCdr<S::Response>,
  sequence_number_counter: SequenceNumber,
}
/*
impl<S: 'static + Service> Client<S> {
  // pub(crate) fn new(node: &mut Node, 
  //   request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Client<S>>
  // {
  //   let request_sender = node.create_publisher
  //     ::<CycloneWrapper<S::Request>>(request_topic, qos.clone())?;
  //   let response_receiver = node.create_subscription
  //     ::<CycloneWrapper<S::Response>>(response_topic, qos)?;
  //   info!("Created new Client: request topic={} response topic={}", request_topic.name(), response_topic.name());

  //   Ok( Client{ request_sender, response_receiver, 
  //               sequence_number_counter: SequenceNumber::new(), })
  // }

  // pub fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
  //   let sequence_number = self.sequence_number_counter;
  //   self.sequence_number_counter = self.sequence_number_counter.next();

  //   // Generate new request id
  //   let request_id = RmwRequestId {
  //     writer_guid: self.request_sender.guid(),
  //     sequence_number,
  //   };

  //   self.request_sender.publish( cyclone_wrap(request_id, request) )?;

  //   Ok( request_id )
  // }

  // pub fn receive_response(&mut self) -> dds::Result<Option<(RmwRequestId,S::Response)>>
  //   where <S as Service>::Response: 'static
  // {
  //   let rwo = self.response_receiver.take()?;
  //   Ok( rwo.map( |(rw, message_info)| 
  //                   cyclone_unwrap(rw, message_info, Some(self.request_sender.guid()))))
  // }

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
*/