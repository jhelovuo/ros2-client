use serde::{Deserialize, Serialize};
use rustdds::*;

/// ROS2 equivalent for DDS GUID
///
/// See https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/Gid.msg
#[derive(
  Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, CdrEncodingSize,
)]
pub struct Gid(
  [u8; 24]
  // Gid definition has changed in ROS 2 from 24 bytes to 16 bytes in Jan 2023
  // https://github.com/ros2/rmw_dds_common/commit/5ab4f5944e4442fe0188e15b10cf11377fb45801
  //
  // This is between Humble (May 2022) and Iron (May 2023)
  //
);

impl From<GUID> for Gid {
  fn from(guid: GUID) -> Self {
    Gid(  std::array::from_fn(|i| *guid.to_bytes().as_ref().get(i).unwrap_or(&0) )  )
  }
}

impl From<Gid> for GUID {
  fn from(gid: Gid) -> GUID {
    GUID::from_bytes( std::array::from_fn(|i| gid.0[i]) )
  }
}


impl Key for Gid {}
