
//! This module defines types to represent ROS 2 names for
//! * Message types, e.g. `std_msgs/String`
//! * Service types, e.g. `turtlesim/Spawn`
//! * action types, e.g. `turtlesim/RotateAbsolute`
//! * 

// TODO:
// Conform fully to https://design.ros2.org/articles/topic_and_service_names.html

pub struct MessageTypeName {
  prefix: String, // typically "msg", but may be "action". What should this part be called?
  //TODO: String is UTF-8, but ROS2 uses just ASCII
  ros2_package_name: String, // or should this be "namespace"?
  ros2_type_name: String,
}

impl MessageTypeName {
  pub fn new(package_name: &str, type_name: &str) -> Self {
    //TODO: Ensure parameters have no leading/trailing slashes
    MessageTypeName {
      prefix: "msg".to_string(),
      ros2_package_name: package_name.to_owned(),
      ros2_type_name: type_name.to_owned(),
    }
  }

  pub(crate) fn new_prefix(package_name: &str, type_name: &str, prefix: String) -> Self {
    MessageTypeName {
      prefix,
      ros2_package_name: package_name.to_owned(),
      ros2_type_name: type_name.to_owned(),
    }
  }

  pub fn package_name(&self) -> &str {
    self.ros2_package_name.as_str()
  }

  pub fn type_name(&self) -> &str {
    self.ros2_type_name.as_str()
  }

  /// Convert to type name used over DDS
  pub fn dds_msg_type(&self) -> String {
    slash_to_colons(self.ros2_package_name.clone() + "/" + &self.prefix + "/dds_/" + &self.ros2_type_name + "_")
  }


}

fn slash_to_colons(s: String) -> String {
  s.replace('/', "::")
}


pub struct ServiceTypeName {
  prefix: String,
  msg: MessageTypeName,
}

impl ServiceTypeName {
  pub fn new(package_name: &str, type_name: &str) -> Self {
    ServiceTypeName {
      prefix: "srv".to_string(),
      msg: MessageTypeName::new(package_name, type_name),
    }
  }

  pub(crate) fn new_prefix(package_name: &str, type_name: &str, prefix: String) -> Self {
    ServiceTypeName {
      prefix,
      msg: MessageTypeName::new(package_name, type_name),
    }
  }

  pub fn package_name(&self) -> &str {
    self.msg.package_name()
  }

  pub fn type_name(&self) -> &str {
    self.msg.type_name()
  }

  pub(crate) fn dds_request_type(&self) -> String {
    slash_to_colons(
      self.package_name().to_owned() + "/" +&self.prefix+ "/dds_/" + &self.type_name() + "_Request_",
    )
  }

  pub(crate) fn dds_response_type(&self) -> String {
    slash_to_colons(
      self.package_name().to_owned() + "/" + &self.prefix + "/dds_/" + &self.type_name() + "_Response_",
    )
  }
}

pub struct ActionTypeName(MessageTypeName);

impl ActionTypeName {
  pub fn new(package_name: &str, type_name: &str) -> Self {
    ActionTypeName( MessageTypeName::new(package_name, type_name) )
  }

  pub fn package_name(&self) -> &str {
    self.0.package_name()
  }

  pub fn type_name(&self) -> &str {
    self.0.type_name()
  }

  pub(crate) fn dds_action_topic(&self, topic: &str) -> MessageTypeName {
    MessageTypeName::new_prefix( self.package_name(), &(self.type_name().to_owned() + topic), "action".to_owned() )
    //slash_to_colons(self.package_name().to_owned() + "/action/dds_/" + &self.type_name())
  }

  pub(crate) fn dds_action_service(&self, srv: &str) -> ServiceTypeName {
    ServiceTypeName::new_prefix( self.package_name(), &(self.type_name().to_owned() + srv), "action".to_owned() )
  }

}


// -------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------




