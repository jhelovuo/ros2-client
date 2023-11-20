use serde::{Deserialize, Serialize};
use rustdds::{rpc::*, GUID};
pub use rustdds::SequenceNumber;

/// [Original](https://docs.ros2.org/foxy/api/rmw/structrmw__request__id__t.html)
/// This structure seems to be identical in structure and function to
/// SampleIdentity defined by the RPC over DDS Spec.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RmwRequestId {
  pub writer_guid: GUID,
  pub sequence_number: SequenceNumber,
}

impl From<RmwRequestId> for SampleIdentity {
  fn from(
    RmwRequestId {
      writer_guid,
      sequence_number,
    }: RmwRequestId,
  ) -> SampleIdentity {
    SampleIdentity {
      writer_guid,
      sequence_number,
    }
  }
}

impl From<SampleIdentity> for RmwRequestId {
  fn from(
    SampleIdentity {
      writer_guid,
      sequence_number,
    }: SampleIdentity,
  ) -> RmwRequestId {
    RmwRequestId {
      writer_guid,
      sequence_number,
    }
  }
}

// [original](https://docs.ros2.org/foxy/api/rmw/structrmw__service__info__t.html)
// But where is this used?
//
// pub struct RmwServiceInfo {
//   pub source_timestamp: RmwTimePointValue,
//   pub received_timestamp: RmwTimePointValue,
//   pub request_id: RmwRequestId,
// }
