use std::marker::PhantomData;

#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rustdds::{rpc::*, *};
use serde::{Deserialize, Serialize};

use crate::message::Message;
use super::{request_id::*, *};

// --------------------------------------------
// --------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct EnhancedWrapper<R> {
  // Enhanced mode does not use any header in the DDS payload.
  // Therefore, we use a wrapper that is identical to the payload.
  response_or_request: R, // ROS2 payload
}
impl<R: Message> Message for EnhancedWrapper<R> {}

pub struct EnhancedServiceMapping<S: Service> {
  phantom: PhantomData<S>,
}

pub type EnhancedServer<S> = ServerGeneric<S, EnhancedServiceMapping<S>>;
pub type EnhancedClient<S> = ClientGeneric<S, EnhancedServiceMapping<S>>;

// Enhanced mode needs no client state in RMW, thus a unit struct.
pub struct EnhancedClientState {}

impl EnhancedClientState {
  pub fn new(_client_guid: GUID) -> EnhancedClientState {
    EnhancedClientState {}
  }
}

impl<S> ServiceMapping<S> for EnhancedServiceMapping<S>
where
  S: Service,
  S::Request: Clone,
{
  type RequestWrapper = EnhancedWrapper<S::Request>;
  type ResponseWrapper = EnhancedWrapper<S::Response>;

  fn unwrap_request(
    wrapped: &Self::RequestWrapper,
    sample_info: &SampleInfo,
  ) -> (RmwRequestId, S::Request) {
    (
      RmwRequestId::from(sample_info.sample_identity()),
      wrapped.response_or_request.clone(),
    )
  }

  fn wrap_response(
    r_id: RmwRequestId,
    response: S::Response,
  ) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    (
      EnhancedWrapper {
        response_or_request: response,
      },
      Some(SampleIdentity::from(r_id)),
    )
  }

  type ClientState = EnhancedClientState;

  fn wrap_request(
    _state: &mut Self::ClientState,
    request: S::Request,
  ) -> (Self::RequestWrapper, Option<RmwRequestId>) {
    (
      EnhancedWrapper {
        response_or_request: request,
      },
      None,
    )
  }

  fn request_id_after_wrap(
    _state: &mut Self::ClientState,
    write_result: SampleIdentity,
  ) -> RmwRequestId {
    RmwRequestId::from(write_result)
  }

  fn unwrap_response(
    _state: &mut Self::ClientState,
    wrapped: Self::ResponseWrapper,
    sample_info: SampleInfo,
  ) -> (RmwRequestId, S::Response) {
    let r_id = sample_info
      .related_sample_identity()
      .map(RmwRequestId::from)
      .unwrap_or_default();

    (r_id, wrapped.response_or_request)
  }

  fn new_client_state(_request_sender: GUID) -> Self::ClientState {
    EnhancedClientState {}
  }
}
