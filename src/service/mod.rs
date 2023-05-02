use std::{
  io,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use mio::{Evented, Poll, PollOpt, Ready, Token};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rustdds::{rpc::*, *};

use crate::{
  message::Message,
  node::Node,
  pubsub::{Publisher, Subscription, MessageInfo},
};

pub mod request_id;

pub mod basic;
pub mod cyclone;
pub mod enhanced;

pub use request_id::*;

// --------------------------------------------
// --------------------------------------------

/// Service trait pairs the Request and Response types together.
/// Additonally, it ensures that Response and Request are Messages
/// (serializable) and we have a means to name the types.
pub trait Service {
  type Request: Message;
  type Response: Message;
  fn request_type_name(&self) -> &str;
  fn response_type_name(&self) -> &str;
}

// --------------------------------------------
// --------------------------------------------

/// AService is a means of constructing a descriptor for a Service on the fly.
/// This allows generic code to construct a Service from the types of
/// request and response. 
pub struct AService<Q,S> 
where
  Q : Message,
  S : Message,
{
  q : PhantomData<Q>,
  s : PhantomData<S>,
  req_type_name : String,
  resp_type_name : String,
}

impl<Q,S> AService<Q,S> 
where
  Q : Message,
  S : Message,
{
  pub fn new(req_type_name: String, resp_type_name: String) -> Self {
    Self {
      req_type_name, 
      resp_type_name,
      q : PhantomData, 
      s: PhantomData,
    }
  }
}

impl<Q,S> Service for AService<Q,S>
where
  Q : Message,
  S : Message,
{
  type Request = Q;
  type Response = S;

  fn request_type_name(&self) -> &str {
    &self.req_type_name
  }

  fn response_type_name(&self) -> &str {
    &self.resp_type_name
  }
}


// --------------------------------------------
// --------------------------------------------


/// Server trait defines the behavior for a "Server". It is required so that we
/// can hide away the ServiceMapping in a Server
pub trait ServerT<S>: Evented
where
  S: Service,
{
  fn receive_request(&self) -> dds::Result<Option<(RmwRequestId, S::Request)>>;
  fn send_response(&self, id: RmwRequestId, response: S::Response) -> dds::Result<()>;
}

// --------------------------------------------
// --------------------------------------------
/// Server end of a ROS2 Service
pub struct Server<S> {
  pub(crate) inner: Box<dyn ServerT<S>>,
}

impl<S> Deref for Server<S> {
  type Target = dyn ServerT<S>;
  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

impl<S> DerefMut for Server<S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut *self.inner
  }
}

impl<S> ServerT<S> for Server<S>
where
  S: 'static + Service,
{
  fn receive_request(&self) -> dds::Result<Option<(RmwRequestId, S::Request)>> {
    self.inner.receive_request()
  }

  fn send_response(&self, id: RmwRequestId, response: S::Response) -> dds::Result<()> {
    self.inner.send_response(id, response)
  }
}

impl<S> Evented for Server<S>
where
  S: 'static + Service,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.inner.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self.inner.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.inner.deregister(poll)
  }
}

/// Client trait defines the behavior for a "Client". It is required so that we
/// can hide away the ServiceMapping in a Client
pub trait ClientT<S>: Evented
where
  S: Service,
{
  fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId>;
  fn receive_response(&self) -> dds::Result<Option<(RmwRequestId, S::Response)>>;
}

/// Client end of a ROS2 Service
pub struct Client<S> {
  pub(crate) inner: Box<dyn ClientT<S>>,
}

// impl<S> Client<S> {
//   pub async fn async_call(request: S::Request) -> dds::Result<S::Response> {
//     let req_id = self.send_request(request)?; // TODO: async write here
//     // async read response

//   }
// }

impl<S> Deref for Client<S> {
  type Target = dyn ClientT<S>;
  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

impl<S> DerefMut for Client<S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut *self.inner
  }
}

impl<S> ClientT<S> for Client<S>
where
  S: 'static + Service,
{
  fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    self.inner.send_request(request)
  }

  fn receive_response(&self) -> dds::Result<Option<(RmwRequestId, S::Response)>> {
    self.inner.receive_response()
  }
}

impl<S> Evented for Client<S>
where
  S: 'static + Service,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.inner.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self.inner.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.inner.deregister(poll)
  }
}

// --------------------------------------------------------------------------------
// --------------------------------------------------------------------------------

// See Spec RPC over DDS Section "7.2.4 Basic and Enhanced Service Mapping for
// RPC over DDS"
pub trait ServiceMapping<S>
where
  S: Service,
  Self::RequestWrapper: Message,
  Self::ResponseWrapper: Message,
{
  type RequestWrapper;
  type ResponseWrapper;

  // Server operations
  fn unwrap_request(
    wrapped: &Self::RequestWrapper,
    sample_info: &MessageInfo,
  ) -> (RmwRequestId, S::Request);
  // Unwrapping will clone the request
  // This is reasonable, because we may have to take it out of another struct
  fn wrap_response(
    r_id: RmwRequestId,
    response: S::Response,
  ) -> (Self::ResponseWrapper, Option<SampleIdentity>);

  // Client operations
  type ClientState;
  // ClientState persists between requests.
  fn new_client_state(request_sender: GUID) -> Self::ClientState;

  // If wrap_requests returns request id, then that will be used. If None, then
  // use return value from request_id_after_wrap
  fn wrap_request(
    state: &mut Self::ClientState,
    request: S::Request,
  ) -> (Self::RequestWrapper, Option<RmwRequestId>);
  fn request_id_after_wrap(
    state: &Self::ClientState,
    write_result: SampleIdentity,
  ) -> RmwRequestId;
  fn unwrap_response(
    state: &Self::ClientState,
    wrapped: Self::ResponseWrapper,
    sample_info: MessageInfo,
  ) -> (RmwRequestId, S::Response);
}

// --------------------------------------------
// --------------------------------------------

pub struct ServerGeneric<S, SW>
where
  S: Service,
  SW: ServiceMapping<S>,
{
  request_receiver: Subscription<SW::RequestWrapper>,
  response_sender: Publisher<SW::ResponseWrapper>,
  phantom: PhantomData<SW>,
}

impl<S, SW> ServerGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  pub(crate) fn new(
    node: &mut Node,
    request_topic: &Topic,
    response_topic: &Topic,
    qos_request: Option<QosPolicies>,
    qos_response: Option<QosPolicies>,
  ) -> dds::Result<ServerGeneric<S, SW>> {
    let request_receiver =
      node.create_subscription::<SW::RequestWrapper>(request_topic, qos_request)?;
    let response_sender =
      node.create_publisher::<SW::ResponseWrapper>(response_topic, qos_response)?;

    info!(
      "Created new ServerGeneric: requests={} response={}",
      request_topic.name(),
      response_topic.name()
    );

    Ok(ServerGeneric {
      request_receiver,
      response_sender,
      phantom: PhantomData,
    })
  }
}

impl<S, SW> ServerT<S> for ServerGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  fn receive_request(&self) -> dds::Result<Option<(RmwRequestId, S::Request)>>
  where
    <S as Service>::Request: 'static,
  {
    let next_sample = self.request_receiver.take()?;

    Ok(next_sample.map(|(s, mi)| SW::unwrap_request(&s, &mi)))
  }

  fn send_response(&self, id: RmwRequestId, response: S::Response) -> dds::Result<()> {
    let (wrapped_response, rsi_opt) = SW::wrap_response(id, response);
    let write_opt = WriteOptionsBuilder::new().related_sample_identity_opt(rsi_opt);
    self
      .response_sender
      .publish_with_options(wrapped_response, write_opt.build())?;
    Ok(())
  }
}

impl<S, SW> Evented for ServerGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.request_receiver.register(poll, token, interest, opts)
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

pub struct ClientGeneric<S, SW>
where
  S: Service,
  SW: ServiceMapping<S>,
{
  request_sender: Publisher<SW::RequestWrapper>,
  response_receiver: Subscription<SW::ResponseWrapper>,
  client_state: SW::ClientState,
  phantom: PhantomData<SW>,
}

impl<S, SW> ClientGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  pub(crate) fn new(
    node: &mut Node,
    request_topic: &Topic,
    response_topic: &Topic,
    qos_request: Option<QosPolicies>,
    qos_response: Option<QosPolicies>,
  ) -> dds::Result<ClientGeneric<S, SW>> {
    let request_sender = node.create_publisher::<SW::RequestWrapper>(request_topic, qos_request)?;
    let response_receiver =
      node.create_subscription::<SW::ResponseWrapper>(response_topic, qos_response)?;
    info!(
      "Created new ClientGeneric: request topic={} response topic={}",
      request_topic.name(),
      response_topic.name()
    );

    let request_sender_guid = request_sender.guid();
    Ok(ClientGeneric {
      request_sender,
      response_receiver,
      client_state: SW::new_client_state(request_sender_guid),
      phantom: PhantomData,
    })
  }
}

impl<S, SW> ClientT<S> for ClientGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  fn send_request(&mut self, request: S::Request) -> dds::Result<RmwRequestId> {
    let (wrapped, rsi_opt) = SW::wrap_request(&mut self.client_state, request);
    let write_opt =
      WriteOptionsBuilder::new().related_sample_identity_opt(rsi_opt.map(SampleIdentity::from));
    let sample_id = self
      .request_sender
      .publish_with_options(wrapped, write_opt.build())?;
    Ok(SW::request_id_after_wrap(&self.client_state, sample_id))
  }

  fn receive_response(&self) -> dds::Result<Option<(RmwRequestId, S::Response)>>
  where
    <S as Service>::Response: 'static,
  {
    let next_sample = self.response_receiver.take()?;

    Ok(next_sample.map(|(rw,msg_info)| {
        SW::unwrap_response(&self.client_state, rw , msg_info)
      }
    ))
  }
}

impl<S, SW> Evented for ClientGeneric<S, SW>
where
  S: 'static + Service,
  SW: 'static + ServiceMapping<S>,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.response_receiver.register(poll, token, interest, opts)
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
