use std::marker::PhantomData;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::message::Message;

use rustdds::*;
use rustdds::rpc::*;

use serde::{Serialize, Deserialize,};

use super::*;

// --------------------------------------------
// --------------------------------------------
// Basic mode header is specified in
// RPC over DDS Section "7.5.1.1.1 Common Types"

#[derive(Serialize,Deserialize)]
pub struct BasicRequestWrapper<R> {
  // "struct RequestHeader":
  request_id: SampleIdentity,
  instance_name: String, // This is apparenlty not used: Always sent as empty string.
  // ROS2 payload  
  request: R,  
}
impl<R:Message> Message for BasicRequestWrapper<R> {}

#[derive(Serialize,Deserialize)]
pub struct BasicResponseWrapper<R> {
  // "struct ReplyHeader":
  related_request_id: SampleIdentity,
  remote_exception_code: u32, // It is uncertain if this is ever used. Transmitted as zero ("REMOTE_EX_OK").
  // ROS2 payload  
  response: R,  
}
impl<R:Message> Message for BasicResponseWrapper<R> {}


pub struct BasicServiceMapping<Q,P> 
{
  request_phantom: PhantomData<Q>,
  response_phantom: PhantomData<P>,
}

pub type BasicServer<S> 
  = Server<S,BasicServiceMapping<<S as Service>::Request,<S as Service>::Response>>;
pub type BasicClient<S> 
  = Client<S,BasicServiceMapping<<S as Service>::Request,<S as Service>::Response>>;

pub struct BasicClientState {
  client_guid: GUID,
  sequence_number_counter: super::SequenceNumber,
}

impl BasicClientState {
  pub fn new(client_guid: GUID) -> BasicClientState {
    BasicClientState { 
      client_guid,
      sequence_number_counter: super::SequenceNumber::default(),
    }
  }
}

impl<Q,P> ServiceMapping<Q,P> for BasicServiceMapping<Q,P> 
where
  Q: Message + Clone,
  P: Message,
{

  type RequestWrapper = BasicRequestWrapper<Q>;
  type ResponseWrapper = BasicResponseWrapper<P>;

  fn unwrap_request(wrapped: &Self::RequestWrapper, _sample_info: &SampleInfo) -> (RmwRequestId, Q) {

    ( RmwRequestId::from(wrapped.request_id) , wrapped.request.clone() )
  }

  fn wrap_response(r_id: RmwRequestId, response:P) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    ( BasicResponseWrapper {
        related_request_id: SampleIdentity::from(r_id),
        remote_exception_code: 0,
        response 
      }, 
      Some(SampleIdentity::from(r_id))
    )
  }


  type ClientState = BasicClientState;

  fn wrap_request(state: &mut Self::ClientState, request:Q) -> (Self::RequestWrapper,Option<RmwRequestId>) {
    state.sequence_number_counter = state.sequence_number_counter.next();

    let rmw_request_id = RmwRequestId {
        writer_guid: state.client_guid,
        sequence_number: state.sequence_number_counter, 
      };

    ( BasicRequestWrapper { 
        request_id: SampleIdentity::from(rmw_request_id),
        instance_name: "".to_string(),
        request, 
      }, 
      Some( rmw_request_id )
    )
  }

  fn request_id_after_wrap(state: &mut Self::ClientState, _write_result:SampleIdentity) -> RmwRequestId {
    // Request id is what we generated into header. 
    // write_result is irrelevant, so we discard it.
    RmwRequestId {
      writer_guid: state.client_guid,
      sequence_number: state.sequence_number_counter,
    }
  }

  fn unwrap_response(_state: &mut Self::ClientState, wrapped: Self::ResponseWrapper, _sample_info: SampleInfo) 
    -> (RmwRequestId, P) 
  {
    let r_id = 
      RmwRequestId {
        writer_guid: wrapped.related_request_id.writer_guid,
        sequence_number: super::SequenceNumber::from( wrapped.related_request_id.sequence_number ),
      };

    ( r_id, wrapped.response )
  }

  fn new_client_state(request_sender: GUID) -> Self::ClientState {
    BasicClientState::new(request_sender)
  }
}
