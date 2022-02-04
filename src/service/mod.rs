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

pub mod request_id;

//pub mod basic;
pub mod enhanced;
//pub mod cyclone;

pub use request_id::*;

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
pub trait ServiceMapping<S> 
where
  S: Service, 
  Self::RequestWrapper: Message,
  Self::ResponseWrapper: Message,
{
  type RequestWrapper;
  type ResponseWrapper;

  // Server operations
  fn unwrap_request(wrapped: &Self::RequestWrapper, sample_info: &SampleInfo) -> (RmwRequestId, S::Request);
  // Unwrapping will clone the request
  // This is reasonable, because we may have to take it out of another struct
  fn wrap_response(r_id: RmwRequestId, response:S::Response) -> (Self::ResponseWrapper, Option<SampleIdentity>);

  // Client operations
  type ClientState;
  // ClientState persists between requests.
  fn new_client_state(request_sender: GUID) -> Self::ClientState;

  // If wrap_requests returns request id, then that will be used. If None, then use
  // return value from request_id_after_wrap
  fn wrap_request(state: &mut Self::ClientState, request:S::Request) -> (Self::RequestWrapper, Option<RmwRequestId>);
  fn request_id_after_wrap(state: &mut Self::ClientState, write_result:SampleIdentity) -> RmwRequestId;
  fn unwrap_response(state: &mut Self::ClientState, wrapped: Self::ResponseWrapper, sample_info: SampleInfo) -> (RmwRequestId, S::Response);
}

// --------------------------------------------
// --------------------------------------------

pub struct Server<S, SW>
where
  S: Service,
  SW: ServiceMapping<S>,
{
  request_receiver: Subscription<SW::RequestWrapper>,
  response_sender: Publisher<SW::ResponseWrapper>,
  phantom: PhantomData<SW>,
}


impl<S,SW> Server<S,SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) 
    -> dds::Result<Server<S,SW>>
  {

    let request_receiver = node
      .create_subscription::<SW::RequestWrapper>(request_topic, qos.clone())?;
    let response_sender = node
      .create_publisher::<SW::ResponseWrapper>(response_topic, qos)?;

    info!("Created new Server: requests={} response={}", request_topic.name(), response_topic.name());

    Ok(Server { request_receiver, response_sender, phantom:PhantomData })
  }

  pub fn receive_request(&mut self) -> dds::Result<Option<(RmwRequestId,S::Request)>>
    where <S as Service>::Request: 'static
  {
    let next_sample = self.request_receiver.take()?;

    Ok( next_sample.map( |(s,mi)| SW::unwrap_request(&s, &mi.sample_info ) ) )
  }

  pub fn send_response(&self, id:RmwRequestId, response: S::Response) -> dds::Result<()> {
    let (wrapped_response, rsi_opt) = SW::wrap_response(id, response);
    let write_opt = WriteOptionsBuilder::new().related_sample_identity_opt(rsi_opt);
    self.response_sender.publish_with_options(wrapped_response, write_opt.build() )?;
    Ok(())
  }
}


impl<S,SW> Evented for Server<S,SW> 
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
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

pub struct Client<S,SW> 
where
  S: Service,
  SW: ServiceMapping<S>,
{
  request_sender: Publisher<SW::RequestWrapper>,
  response_receiver: Subscription<SW::ResponseWrapper>,
  client_state: SW::ClientState,
  phantom: PhantomData<SW>,
}

impl<S,SW> Client<S,SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  pub(crate) fn new(node: &mut Node, 
    request_topic: &Topic, response_topic: &Topic, qos:Option<QosPolicies>) -> dds::Result<Client<S,SW>>
  {
    let request_sender = node.create_publisher
      ::<SW::RequestWrapper>(request_topic, qos.clone())?;
    let response_receiver = node.create_subscription
      ::<SW::ResponseWrapper>(response_topic, qos)?;
    info!("Created new Client: request topic={} response topic={}", request_topic.name(), response_topic.name());

    let request_sender_guid = request_sender.guid();
    Ok( Client{ request_sender, response_receiver, 
                client_state: SW::new_client_state( request_sender_guid ), 
                phantom: PhantomData,
              })
  }

  pub fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    let (wrapped,rsi_opt) = SW::wrap_request(&mut self.client_state, request);
    let write_opt = WriteOptionsBuilder::new().related_sample_identity_opt(  rsi_opt.map(SampleIdentity::from));
    let sample_id = self.request_sender.publish_with_options( wrapped , write_opt.build() )?;
    Ok( SW::request_id_after_wrap(&mut self.client_state, sample_id) )
  }

  pub fn receive_response(&mut self) -> dds::Result<Option<(RmwRequestId,S::Response)>>
    where <S as Service>::Response: 'static
  {
    let next_sample = self.response_receiver.take()?;

    Ok( next_sample.map( |(s,mi)| SW::unwrap_response(&mut self.client_state, s, mi.sample_info ) ) )
  }

}


impl<S,SW> Evented for Client<S,SW> 
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
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
