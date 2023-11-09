use serde::{Deserialize, Serialize};
use rustdds::*;

/// ROS2 equivalent for DDS GUID
///
/// See https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/Gid.msg
#[derive(
  Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, CdrEncodingSize,
)]
pub struct Gid([u8; 16]);

impl From<GUID> for Gid {
  fn from(guid: GUID) -> Self {
    Gid( guid.to_bytes() )
  }
}

impl From<Gid> for GUID {
  fn from(gid: Gid) -> GUID {
    GUID::from_bytes(gid.0)
  }
}


impl Key for Gid {}
