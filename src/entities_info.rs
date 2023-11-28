//! Message types for ROS 2 Discovery
//!
//! For background, see
//! [Node to Participant mapping](https://design.ros2.org/articles/Node_to_Participant_mapping.html)

use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use crate::{
  gid::Gid,
  names::{NameError, NodeName},
};

// Each time a Node adds/removes a Reader or Writer (Publisher / Subscrption in
// ROS terms) is must publish a new ParticipantEntitiesInfo that describes its
// current composition. This overwrites the previsous ParticipantEntitiesInfo.

/// Information structure for other DomainParticipants in a ROS 2 network
///
/// See [ParticipantEntitiesInfo](https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/ParticipantEntitiesInfo.msg) in ROS2.
///
/// Gives a list of ROS 2 nodes that are represented by a DDS DomainParticipant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEntitiesInfo {
  pub(crate) gid: Gid, // GUID of a DomainParticipant
  pub(crate) node_entities_info_seq: Vec<NodeEntitiesInfo>,
  // ^ ROS 2 Nodes implemented by the DomainParticipant.
}

impl ParticipantEntitiesInfo {
  pub fn new(gid: Gid, node_entities_info_seq: Vec<NodeEntitiesInfo>) -> ParticipantEntitiesInfo {
    ParticipantEntitiesInfo {
      gid,
      node_entities_info_seq,
    }
  }

  pub fn gid(&self) -> Gid {
    self.gid
  }

  pub fn nodes(&self) -> &Vec<NodeEntitiesInfo> {
    &self.node_entities_info_seq
  }
}

/// Information about a node in ROS2 network
///
/// See [NodeEntitiesInfo](https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/NodeEntitiesInfo.msg)
///
/// Defines a ROS 2 Node and how it is mapped to DDS entities.
///
/// Consists of name and namespace definitions, and lists of
/// Reader and Writer ids that belong to this Node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "repr::NodeEntitiesInfo", into = "repr::NodeEntitiesInfo")]
pub struct NodeEntitiesInfo {
  name: NodeName,
  reader_gid_seq: Vec<Gid>,
  writer_gid_seq: Vec<Gid>,
}

impl NodeEntitiesInfo {
  pub fn new(name: NodeName) -> NodeEntitiesInfo {
    NodeEntitiesInfo {
      name,
      reader_gid_seq: Vec::new(),
      writer_gid_seq: Vec::new(),
    }
  }

  pub fn namespace(&self) -> &str {
    self.name.namespace()
  }

  pub fn name(&self) -> &str {
    self.name.base_name()
  }

  /// Full name of the node namespace + name eg. /some_node
  pub fn fully_qualified_name(&self) -> String {
    self.name.fully_qualified_name()
  }

  pub fn add_writer(&mut self, gid: Gid) {
    if !self.writer_gid_seq.contains(&gid) {
      self.writer_gid_seq.push(gid);
    }
  }

  pub fn add_reader(&mut self, gid: Gid) {
    if !self.reader_gid_seq.contains(&gid) {
      self.reader_gid_seq.push(gid);
    }
  }
}

impl TryFrom<repr::NodeEntitiesInfo> for NodeEntitiesInfo {
  type Error = NameError;

  fn try_from(r: repr::NodeEntitiesInfo) -> Result<NodeEntitiesInfo, NameError> {
    let name = NodeName::new(&r.node_namespace, &r.node_name)?;
    Ok(NodeEntitiesInfo {
      name,
      reader_gid_seq: r.reader_gid_seq,
      writer_gid_seq: r.writer_gid_seq,
    })
  }
}

impl From<NodeEntitiesInfo> for repr::NodeEntitiesInfo {
  fn from(n: NodeEntitiesInfo) -> repr::NodeEntitiesInfo {
    repr::NodeEntitiesInfo {
      node_namespace: n.name.namespace().to_owned(),
      node_name: n.name.base_name().to_owned(),
      reader_gid_seq: n.reader_gid_seq,
      writer_gid_seq: n.writer_gid_seq,
    }
  }
}

pub(crate) mod repr {
  use serde::{Deserialize, Serialize};

  use crate::gid::Gid;

  #[derive(Clone, Serialize, Deserialize)]
  pub(crate) struct NodeEntitiesInfo {
    // Field names are from .msg definition
    pub node_namespace: String, // original .msg specifies .len() <= 256
    pub node_name: String,      // original .msg specifies .len() <= 256
    pub reader_gid_seq: Vec<Gid>,
    pub writer_gid_seq: Vec<Gid>,
  }
}
