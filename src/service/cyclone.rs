use std::{marker::PhantomData, sync::atomic};

#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rustdds::{rpc::*, *};
use serde::{Deserialize, Serialize};

use crate::{message::Message, MessageInfo};
use super::{
  request_id::{RmwRequestId, SequenceNumber},
  ClientGeneric, ServerGeneric, Service, ServiceMapping,
};

// This is reverse-engineered from
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/rmw_node.cpp
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/serdata.hpp
// This is a header that Cyclone puts in DDS messages. Same header is used for
// Requst and Response.
#[derive(Serialize, Deserialize)]
pub struct CycloneWrapper<R> {
  guid_second_half: [u8; 8], // CycolenDDS RMW only sends last 8 bytes of client GUID
  sequence_number_high: i32,
  sequence_number_low: u32,
  response_or_request: R, // ROS2 payload
}
impl<R: Message> Message for CycloneWrapper<R> {}

pub struct CycloneServiceMapping<S> {
  phantom: PhantomData<S>,
}

pub type CycloneServer<S> = ServerGeneric<S, CycloneServiceMapping<S>>;
pub type CycloneClient<S> = ClientGeneric<S, CycloneServiceMapping<S>>;

pub struct CycloneClientState {
  client_guid: GUID,
  sequence_number_counter: atomic::AtomicI64,
  //sequence_number_counter: SequenceNumber,
}

impl CycloneClientState {
  pub fn new(client_guid: GUID) -> CycloneClientState {
    CycloneClientState {
      client_guid,
      sequence_number_counter: atomic::AtomicI64::new(SequenceNumber::default().into()), /* sequence_number_counter: SequenceNumber::zero(), */
    }
  }

  pub fn next_sequence_number(&self) -> SequenceNumber {
    SequenceNumber::from(
      self
        .sequence_number_counter
        .fetch_add(1, atomic::Ordering::Acquire),
    )
  }

  pub fn sequence_number(&self) -> SequenceNumber {
    SequenceNumber::from(self.sequence_number_counter.load(atomic::Ordering::Acquire))
  }
}

impl<S> ServiceMapping<S> for CycloneServiceMapping<S>
where
  S: Service,
  S::Request: Clone,
{
  type RequestWrapper = CycloneWrapper<S::Request>;
  type ResponseWrapper = CycloneWrapper<S::Response>;

  fn unwrap_request(
    wrapped: &Self::RequestWrapper,
    message_info: &MessageInfo,
  ) -> (RmwRequestId, S::Request) {
    let r_id = RmwRequestId {
      writer_guid: message_info.writer_guid(),
      // Last 8 bytes of writer (client) GUID should be in the wrapper also
      sequence_number: SequenceNumber::from_high_low(
        wrapped.sequence_number_high,
        wrapped.sequence_number_low,
      ),
    };

    (r_id, wrapped.response_or_request.clone())
  }

  fn wrap_response(
    r_id: RmwRequestId,
    response: S::Response,
  ) -> (Self::ResponseWrapper, Option<SampleIdentity>) {
    (cyclone_wrap(r_id, response), None)
  }

  type ClientState = CycloneClientState;

  fn wrap_request(
    state: &Self::ClientState,
    request: S::Request,
  ) -> (Self::RequestWrapper, Option<RmwRequestId>) {
    let sequence_number = state.next_sequence_number();

    // Generate new request id
    let request_id = RmwRequestId {
      writer_guid: state.client_guid,
      sequence_number,
    };

    (cyclone_wrap(request_id, request), Some(request_id))
  }

  fn request_id_after_wrap(
    state: &Self::ClientState,
    _write_result: SampleIdentity,
  ) -> RmwRequestId {
    // Request id is what we generated into header.
    // write_result is irrelevant, so we discard it.
    RmwRequestId {
      writer_guid: state.client_guid,
      sequence_number: state.sequence_number(),
    }
  }

  fn unwrap_response(
    state: &Self::ClientState,
    wrapped: Self::ResponseWrapper,
    message_info: MessageInfo,
  ) -> (RmwRequestId, S::Response) {
    let mut client_guid_bytes = [0; 16];
    {
      let (first_half, second_half) = client_guid_bytes.split_at_mut(8);

      // this seems a bit odd, but source is
      // https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_impl.cpp
      // function take_response()
      first_half.copy_from_slice(&state.client_guid.to_bytes().as_slice()[0..8]);

      // This is received in the wrapper header
      second_half.copy_from_slice(&message_info.writer_guid().to_bytes()[8..16]);
    }

    let r_id = RmwRequestId {
      writer_guid: GUID::from_bytes(client_guid_bytes),
      sequence_number: SequenceNumber::from_high_low(
        wrapped.sequence_number_high,
        wrapped.sequence_number_low,
      ),
    };

    (r_id, wrapped.response_or_request)
  }

  fn new_client_state(request_sender: GUID) -> Self::ClientState {
    CycloneClientState::new(request_sender)
  }
}

fn cyclone_wrap<R>(r_id: RmwRequestId, response_or_request: R) -> CycloneWrapper<R> {
  let sn = r_id.sequence_number;

  let mut guid_second_half = [0; 8];
  // writer_guid means client GUID (i.e. request writer)
  guid_second_half.copy_from_slice(&r_id.writer_guid.to_bytes()[8..16]);

  CycloneWrapper {
    guid_second_half,
    sequence_number_high: sn.high(),
    sequence_number_low: sn.low(),
    response_or_request,
  }
}
