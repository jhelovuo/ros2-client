use std::io;
use std::marker::PhantomData;

use mio::{Evented, Poll, Token, PollOpt, Ready,};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::message::Message;
use crate::node::Node;
use crate::pubsub::{Publisher, Subscription, };

use rustdds::*;
use rustdds::rpc::*;

use serde::{Serialize, Deserialize,};

//use concat_arrays::concat_arrays;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequenceNumber {
  number: i64,
}

impl SequenceNumber {
  pub fn new(number: i64) -> SequenceNumber { 
    SequenceNumber{ number } 
  }

  pub fn zero() -> SequenceNumber {
    SequenceNumber::new(0)
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

impl Default for SequenceNumber {
  fn default() -> SequenceNumber {
    SequenceNumber::new(1) // This is consistent with RustDDS SequenceNumber default value
  }
}

impl From<SequenceNumber> for i64 {
  fn from(sn:SequenceNumber) -> i64 {
    sn.number
  }
}


/// [Original](https://docs.ros2.org/foxy/api/rmw/structrmw__request__id__t.html)
#[derive(Clone,Copy,Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmwRequestId {
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber, 
}

impl From<RmwRequestId> for SampleIdentity {
  fn from(si : RmwRequestId) -> SampleIdentity {
    SampleIdentity {
      writer_guid: si.writer_guid,
      sequence_number: rustdds::SequenceNumber::from( i64::from( si.sequence_number) ),
    }
  }
}

impl From<SampleIdentity> for RmwRequestId {
  fn from(si : SampleIdentity) -> RmwRequestId {
    RmwRequestId {
      writer_guid: si.writer_guid,
      sequence_number: SequenceNumber::new( i64::from(si.sequence_number ) ),
    }
  }
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

pub trait Service {
    type Request: Message;
    type Response: Message;
    fn request_type_name() -> String;
    fn response_type_name() -> String;
}

// --------------------------------------------
// --------------------------------------------

// See Spec RPC over DDS Section "7.2.4 Basic and Enhanced Service Mapping for RPC over DDS"
pub trait ServiceMapping<Q,P> 
where 
  Self::RequestWrapper: Message,
  Self::ResponseWrapper: Message,
{
  type RequestWrapper;
  type ResponseWrapper;

  // Server operations
  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, Q);
  // Unwrapping will clone the request
  // This is reasonable, because we may have to take it out of another struct
  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>);

  // Client operations
  type ClientState;
  // ClientState persists between requests.
  fn new_client_state(request_sender: GUID) -> Self::ClientState;

  // If wrap_requests returns request id, then that will be used. If None, then use
  // return value from request_id_after_wrap
  fn wrap_request(state: &mut Self::ClientState, request:Q) -> (Self::RequestWrapper, Option<RmwRequestId>);
  fn request_id_after_wrap(state: &mut Self::ClientState, write_result:SampleIdentity) -> RmwRequestId;
  fn unwrap_response(state: &mut Self::ClientState, wrapped: Self::ResponseWrapper, sample_info: SampleInfo) -> (RmwRequestId, P);
}

// --------------------------------------------
// --------------------------------------------

// This is reverse-engineered from
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/rmw_node.cpp
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/serdata.hpp
// This is a header that Cyclone puts in DDS messages. Same header is used for Requst and Response.
#[derive(Serialize,Deserialize)]
pub struct CycloneWrapper<R> {
  guid_second_half: [u8;8], // CycolenDDS RMW only sends last 8 bytes of client GUID
  sequence_number_high: i32,
  sequence_number_low: u32,
  response_or_request: R,  // ROS2 payload  
}
impl<R:Message> Message for CycloneWrapper<R> {}

pub struct CycloneServiceMapping<Q,P> 
{
  request_phantom: PhantomData<Q>,
  response_phantom: PhantomData<P>,
}

pub type CycloneServer<S> 
  = Server<S,CycloneServiceMapping<<S as Service>::Request,<S as Service>::Response>>;
pub type CycloneClient<S> 
  = Client<S,CycloneServiceMapping<<S as Service>::Request,<S as Service>::Response>>;

pub struct CycloneClientState {
 client_guid: GUID,
 sequence_number_counter: SequenceNumber,
}

impl CycloneClientState {
  pub fn new(client_guid: GUID) -> CycloneClientState {
    CycloneClientState {
      client_guid,
      sequence_number_counter: SequenceNumber::zero(),
    }
  }
}

impl<Q,P> ServiceMapping<Q,P> for CycloneServiceMapping<Q,P> 
where
  Q: Message + Clone,
  P: Message,
{

  type RequestWrapper = CycloneWrapper<Q>;
  type ResponseWrapper = CycloneWrapper<P>;

  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, Q) {
    let r_id = RmwRequestId {
      writer_guid: sample_info.writer_guid(),
      // Last 8 bytes of writer (client) GUID should be in the wrapper also
      sequence_number: SequenceNumber::
        from_high_low(wrapped.sequence_number_high, wrapped.sequence_number_low),
    };

    ( r_id, wrapped.response_or_request.clone() )
  }

  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    (cyclone_wrap(r_id,response), None)
  }


  type ClientState = CycloneClientState;

  fn wrap_request(state: &mut Self::ClientState, request:Q) -> (Self::RequestWrapper,Option<RmwRequestId>) {
    state.sequence_number_counter = state.sequence_number_counter.next();

    // Generate new request id
    let request_id = RmwRequestId {
      writer_guid: state.client_guid,
      sequence_number: state.sequence_number_counter,
    };

    (cyclone_wrap(request_id, request) , Some(request_id))
  }

  fn request_id_after_wrap(state: &mut Self::ClientState, _write_result:SampleIdentity) -> RmwRequestId {
    // Request id is what we generated into header. 
    // write_result is irrelevant, so we discard it.
    RmwRequestId {
      writer_guid: state.client_guid,
      sequence_number: state.sequence_number_counter,
    }
  }

  fn unwrap_response(state: &mut Self::ClientState, wrapped: Self::ResponseWrapper, sample_info: SampleInfo) 
    -> (RmwRequestId, P) 
  {
    let mut client_guid_bytes = [0;16];
    {
      let (first_half, second_half) = client_guid_bytes.split_at_mut(8);
  
      // this seems a bit odd, but source is
      // https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_impl.cpp
      // function take_response()
      first_half.copy_from_slice( &state.client_guid.to_bytes().as_slice()[0..8]);
  
      // This is received in the wrapper header
      second_half.copy_from_slice( &sample_info.writer_guid().to_bytes()[8..16] );
    }

    let r_id = RmwRequestId {
      writer_guid: GUID::from_bytes(client_guid_bytes),
      sequence_number: SequenceNumber::
        from_high_low(wrapped.sequence_number_high, wrapped.sequence_number_low),
    };

    ( r_id, wrapped.response_or_request )
  }

  fn new_client_state(request_sender: GUID) -> Self::ClientState {
    CycloneClientState {
      client_guid: request_sender,
      sequence_number_counter: SequenceNumber::zero(),
    }
  }
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

#[derive(Serialize,Deserialize)]
pub struct EnhancedWrapper<R> {
  // Enhanced mode does not use any header in the DDS payload.
  // Therefore, we use a wrapper that is identical to the payload.
  response_or_request: R,  // ROS2 payload  
}
impl<R:Message> Message for EnhancedWrapper<R> {}

pub struct EnhancedServiceMapping<Q,P> 
{
  request_phantom: PhantomData<Q>,
  response_phantom: PhantomData<P>,
}

pub type EnhancedServer<S> 
  = Server<S,EnhancedServiceMapping<<S as Service>::Request,<S as Service>::Response>>;
pub type EnhancedClient<S> 
  = Client<S,EnhancedServiceMapping<<S as Service>::Request,<S as Service>::Response>>;

// Enhanced mode needs no client state in RMW, thus a unit struct.
pub struct EnhancedClientState {}

impl EnhancedClientState {
  pub fn new(_client_guid: GUID) -> EnhancedClientState {
    EnhancedClientState { }
  }
}

impl<Q,P> ServiceMapping<Q,P> for EnhancedServiceMapping<Q,P> 
where
  Q: Message + Clone,
  P: Message,
{

  type RequestWrapper = EnhancedWrapper<Q>;
  type ResponseWrapper = EnhancedWrapper<P>;

  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, Q) {
    ( RmwRequestId::from(sample_info.sample_identity() ) , wrapped.response_or_request.clone() )
  }

  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    (  EnhancedWrapper{ response_or_request: response }, Some(SampleIdentity::from(r_id)))
  }


  type ClientState = EnhancedClientState;

  fn wrap_request(_state: &mut Self::ClientState, request:Q) -> (Self::RequestWrapper,Option<RmwRequestId>) {
    (EnhancedWrapper{ response_or_request: request }, None)
  }

  fn request_id_after_wrap(_state: &mut Self::ClientState, write_result:SampleIdentity) -> RmwRequestId {
    RmwRequestId::from(write_result)
  }

  fn unwrap_response(_state: &mut Self::ClientState, wrapped: Self::ResponseWrapper, sample_info: SampleInfo) 
    -> (RmwRequestId, P) 
  {
    let r_id = 
      sample_info.related_sample_identity()
        .map( RmwRequestId::from )
        .unwrap_or_default();

    ( r_id, wrapped.response_or_request )
  }

  fn new_client_state(_request_sender: GUID) -> Self::ClientState {
    EnhancedClientState { }
  }
}

// --------------------------------------------
// --------------------------------------------

pub struct Server<S:Service, W:ServiceMapping<S::Request,S::Response>>
{
  request_receiver: Subscription<W::RequestWrapper>,
  response_sender: Publisher<W::ResponseWrapper>,
  phantom: PhantomData<S>,
}


impl<S,W> Server<S,W>
where
  S: 'static + Service,
  W: 'static + ServiceMapping<S::Request, S::Response>,
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


impl<S, W> Evented for Server<S,W> 
where
  S: 'static + Service,
  W: 'static + ServiceMapping<S::Request, S::Response>,
{
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

pub struct Client<S:Service, W:ServiceMapping<S::Request,S::Response>> {
  request_sender: Publisher<W::RequestWrapper>,
  response_receiver: Subscription<W::ResponseWrapper>,
  client_state: W::ClientState,
  phantom: PhantomData<S>,
}

impl<S,W> Client<S,W>
where
  S: 'static + Service,
  W: 'static + ServiceMapping<S::Request, S::Response>,
{
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Client<S,W>>
  {
    let request_sender = node.create_publisher
      ::<W::RequestWrapper>(request_topic, qos.clone())?;
    let response_receiver = node.create_subscription
      ::<W::ResponseWrapper>(response_topic, qos)?;
    info!("Created new Client: request topic={} response topic={}", request_topic.name(), response_topic.name());

    let request_sender_guid = request_sender.guid();
    Ok( Client{ request_sender, response_receiver, 
                client_state: W::new_client_state( request_sender_guid ), 
                phantom: PhantomData,
              })
  }

  pub fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    let (wrapped,rsi_opt) = W::wrap_request(&mut self.client_state, request);
    let write_opt = WriteOptionsBuilder::new().related_sample_identity_opt(  rsi_opt.map(SampleIdentity::from));
    let sample_id = self.request_sender.publish_with_options( wrapped , write_opt.build() )?;
    Ok( W::request_id_after_wrap(&mut self.client_state, sample_id) )
  }

  pub fn receive_response(&mut self) -> dds::Result<Option<(RmwRequestId,S::Response)>>
    where <S as Service>::Response: 'static
  {
    let next_sample = self.response_receiver.take()?;

    Ok( next_sample.map( |(s,mi)| W::unwrap_response(&mut self.client_state, s, mi.sample_info ) ) )
  }

}


impl<S,W> Evented for Client<S,W> 
where
  S: 'static + Service,
  W: 'static + ServiceMapping<S::Request, S::Response>,
{
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
