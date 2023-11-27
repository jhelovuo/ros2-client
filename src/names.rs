//! This module defines types to represent ROS 2 names for
//! * Message types, e.g. `std_msgs/String`
//! * Service types, e.g. `turtlesim/Spawn`
//! * action types, e.g. `turtlesim/RotateAbsolute`
//! *

// TODO:
// Conform fully to https://design.ros2.org/articles/topic_and_service_names.html
// and
// https://wiki.ros.org/Names --> Section 1.1.1 Valid Names

/// Names for Nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeName {
  namespace: String,
  base_name: String,
}

impl NodeName {
  pub fn new(namespace: &str, base_name: &str) -> Result<NodeName, NameError> {
    match base_name.chars().next() {
      None => return Err(NameError::Empty),
      Some(c) if c.is_ascii_alphabetic() => { /*ok*/ }
      Some(_other) => return Err(NameError::BadChar),
    }

    if base_name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
      /* ok */
    } else {
      return Err(NameError::BadChar);
    }

    match namespace.chars().next() {
      None => { /* ok */ } // but what does this mean? Same as global namespace "/" ?
      Some(c) if c.is_ascii_alphabetic() || c == '/' => { /*ok*/ }
      // Character '~' is not accepted, because we do not know what that would mean in a Node's
      // name.
      Some(_other) => return Err(NameError::BadChar),
    }

    // TODO: Should we require first char to be exactly '/' ?
    // Otherwise, what would be the absolute node name?
    if namespace
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '/')
    {
      /* ok */
    } else {
      return Err(NameError::BadChar);
    }
    if namespace.ends_with('/') {
      return Err(NameError::BadSlash);
    }

    Ok(NodeName {
      namespace: namespace.to_owned(),
      base_name: base_name.to_owned(),
    })
  }

  pub fn namespace(&self) -> &str {
    &self.namespace
  }
  pub fn base_name(&self) -> &str {
    &self.base_name
  }

  pub fn fully_qualified_name(&self) -> String {
    let mut fqn = self.namespace.clone();
    fqn.push('/');
    fqn.push_str(&self.base_name);
    fqn
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
  Empty,
  BadChar,
  BadSlash,
}

use std::fmt;

impl fmt::Display for NameError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      NameError::Empty => write!(f, "Base name must not be empty"),
      NameError::BadChar => write!(f, "Bad chracters in Name"),
      NameError::BadSlash => write!(f, "Invalid placement of seprator slashes"),
    }
  }
}

/// Names for Topics, Services
///
/// See [Names](https://wiki.ros.org/Names) for ROS 1.
/// and [topic and Service name mapping to DDS](https://design.ros2.org/articles/topic_and_service_names.html)
/// in ROS 2 documentation.
#[allow(dead_code)]
pub struct Name {
  base_name: String, // The last part of the full name. Must not be empty.
  preceeding_tokens: Vec<String>, // without separating slashes
  absolute: bool,    // in string format, absolute names begin with a slash
}

// TODO: We do not (yet) support tilde-expansion or brace-substitutions.

impl Name {
  pub fn parse(namespace: &str, base_name: &str) -> Result<Name, NameError> {
    // TODO: Implement all of the checks here
    let (namespace_rel, absolute) = if let Some(rel) = namespace.strip_prefix('/') {
      (rel, true)
    } else {
      (namespace, false)
    };

    if base_name.is_empty() {
      return Err(NameError::Empty);
    }

    if base_name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_')
      && base_name.starts_with(|c: char| c.is_ascii_alphabetic())
    { /* ok */
    } else {
      return Err(NameError::BadChar);
    }

    let preceeding_tokens = namespace_rel
      .split('/')
      .map(str::to_owned)
      .collect::<Vec<String>>();
    // Starting slash, ending slash, or repeated slash all
    // produce empty strings.

    if preceeding_tokens.iter().any(String::is_empty) {
      return Err(NameError::BadSlash);
    }

    if preceeding_tokens
      .iter()
      .all(|tok| tok.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    { /* ok */
    } else {
      return Err(NameError::BadChar);
    }

    Ok(Name {
      base_name: base_name.to_owned(),
      preceeding_tokens,
      absolute,
    })
  }

  pub fn to_dds_name(&self) -> String {
    todo!()
  }
}

/// Name for `.msg` type, or a data type carried over a Topic.
///
/// This would be called a "Pacakge Resource Name", at least in ROS 1.
///
/// Note that this is not for naming Topics, but data types of Topics.
///
/// See [Names](https://wiki.ros.org/Names) Section 1.2 Package Resource Names.
pub struct MessageTypeName {
  prefix: String, // typically "msg", but may be "action". What should this part be called?
  //TODO: String is strictly UTF-8, but ROS2 uses just byte strings that are recommended to be
  // UTF-8
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
    slash_to_colons(
      self.ros2_package_name.clone() + "/" + &self.prefix + "/dds_/" + &self.ros2_type_name + "_",
    )
  }
}

fn slash_to_colons(s: String) -> String {
  s.replace('/', "::")
}

/// Similar to [`MessageTypeName`], but names a Service type.
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
      self.package_name().to_owned()
        + "/"
        + &self.prefix
        + "/dds_/"
        + self.type_name()
        + "_Request_",
    )
  }

  pub(crate) fn dds_response_type(&self) -> String {
    slash_to_colons(
      self.package_name().to_owned()
        + "/"
        + &self.prefix
        + "/dds_/"
        + self.type_name()
        + "_Response_",
    )
  }
}

/// Similar to [`MessageTypeName`], but names an Action type.
pub struct ActionTypeName(MessageTypeName);

impl ActionTypeName {
  pub fn new(package_name: &str, type_name: &str) -> Self {
    ActionTypeName(MessageTypeName::new(package_name, type_name))
  }

  pub fn package_name(&self) -> &str {
    self.0.package_name()
  }

  pub fn type_name(&self) -> &str {
    self.0.type_name()
  }

  pub(crate) fn dds_action_topic(&self, topic: &str) -> MessageTypeName {
    MessageTypeName::new_prefix(
      self.package_name(),
      &(self.type_name().to_owned() + topic),
      "action".to_owned(),
    )
    //slash_to_colons(self.package_name().to_owned() + "/action/dds_/" +
    // &self.type_name())
  }

  pub(crate) fn dds_action_service(&self, srv: &str) -> ServiceTypeName {
    ServiceTypeName::new_prefix(
      self.package_name(),
      &(self.type_name().to_owned() + srv),
      "action".to_owned(),
    )
  }
}

// -------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------
