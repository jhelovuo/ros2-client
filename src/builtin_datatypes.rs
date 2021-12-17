use serde::{Deserialize, Serialize};

use rustdds::*;



/// Rosout message structure, received from RosParticipant rosout reader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
  timestamp: Timestamp,
  level: u8,
  name: String,
  msg: String,
  file: String,
  function: String,
  line: u32,
}

impl Log {
  /// Timestamp when rosout message was sent
  pub fn get_timestamp(&self) -> &Timestamp {
    &self.timestamp
  }

  /// Rosout level
  pub fn get_level(&self) -> u8 {
    self.level
  }

  /// Name of the rosout message
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Actual message
  pub fn get_msg(&self) -> &str {
    &self.msg
  }

  pub fn get_file(&self) -> &str {
    &self.file
  }

  pub fn get_function(&self) -> &str {
    &self.function
  }

  pub fn get_line(&self) -> u32 {
    self.line
  }
}
