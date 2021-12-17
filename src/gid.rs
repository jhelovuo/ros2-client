use serde::{Deserialize, Serialize};

use rustdds::*;

/// ROS2 equivalent for DDS GUID
///
/// See https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/Gid.msg
#[derive(
  Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, CdrEncodingSize,
)]
pub struct Gid {
  data: [u8; 24],
}

impl Gid {
  pub fn from_guid(guid: GUID) -> Gid {
    let mut data: [u8; 24] = [0; 24];
    data[..12].clone_from_slice(guid.guid_prefix.as_slice());
    data[12..15].clone_from_slice(&guid.entity_id.entity_key);
    data[15] = u8::from(guid.entity_id.entity_kind);
    Gid { data }
  }
}

impl Key for Gid {}
