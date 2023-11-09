use serde::{Deserialize, Serialize};

use crate::gid::Gid;

// For background, see
// https://design.ros2.org/articles/Node_to_Participant_mapping.html

// Each time a Node adds/removes a Reader or Writer (Publisher / Subscrption in ROS terms)
// is must publish a new ParticipantEntitiesInfo that describes its current composition.
// This overwrites the previsous ParticipantEntitiesInfo.


/// Information structure for other DomainParticipants in ROS2 network
///
/// See [ParticipantEntitiesInfo](https://github.com/ros2/rmw_dds_common/blob/master/rmw_dds_common/msg/ParticipantEntitiesInfo.msg) in ROS2.
///
/// Gives a list of ROS 2 nodes that are represented by a DDS DomainPArticipant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEntitiesInfo {
  gid: Gid, // GUID of a DomainParticipant
  node_entities_info_seq: Vec<NodeEntitiesInfo>, // ROS 2 Nodes implemented by the DomainParticipant
  // field names from .msg definition
}

impl ParticipantEntitiesInfo {
  pub fn new(gid: Gid, node_entities_info_seq: Vec<NodeEntitiesInfo>) -> ParticipantEntitiesInfo {
    ParticipantEntitiesInfo { gid, node_entities_info_seq }
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
pub struct NodeEntitiesInfo {
  node_namespace: String, // original .msg specifies .len() <= 256
  node_name: String,      // original .msg specifies .len() <= 256
  reader_gid_seq: Vec<Gid>,
  writer_gid_seq: Vec<Gid>,
}

impl NodeEntitiesInfo {
  pub fn new(name: String, namespace: String) -> NodeEntitiesInfo {
    NodeEntitiesInfo {
      node_namespace: namespace,
      node_name: name,
      reader_gid_seq: Vec::new(),
      writer_gid_seq: Vec::new(),
    }
  }

  pub fn namespace(&self) -> &str {
    &self.node_namespace
  }

  pub fn name(&self) -> &str {
    &self.node_name
  }

  /// Full name of the node namespace + name eg. /some_node
  pub fn get_full_name(&self) -> String {
    let mut name = self.node_namespace.clone();
    name.push('/');
    name.push_str(&self.node_name);
    name
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
