use rustdds::{
  rpc::SampleIdentity,
  *,
};

// This is just a thinly veiled RustDDS SampleInfo
#[derive(Debug, Clone)]
pub struct MessageInfo {
  received_timestamp: Timestamp,
  source_timestamp: Option<Timestamp>,
  sequence_number: SequenceNumber,
  publisher: GUID,
  related_sample_identity: Option<SampleIdentity>,
}

impl MessageInfo {
  pub fn received_timestamp(&self) -> Timestamp {
    self.received_timestamp
  }

  pub fn source_timestamp(&self) -> Option<Timestamp> {
    self.source_timestamp
  }

  pub fn writer_guid(&self) -> GUID {
    self.publisher
  }

  pub fn sample_identity(&self) -> rustdds::rpc::SampleIdentity {
    rustdds::rpc::SampleIdentity {
      writer_guid: self.writer_guid(),
      sequence_number: self.sequence_number,
    }
  }

  pub fn related_sample_identity(&self) -> Option<SampleIdentity> {
    self.related_sample_identity
  }
}

impl From<&SampleInfo> for MessageInfo {
  fn from(sample_info: &SampleInfo) -> MessageInfo {
    MessageInfo {
      received_timestamp: Timestamp::ZERO, // TODO!
      source_timestamp: sample_info.source_timestamp(),
      sequence_number: sample_info.sample_identity().sequence_number,
      publisher: sample_info.publication_handle(), // DDS has an odd name for this
      related_sample_identity: sample_info.related_sample_identity(),
    }
  }
}

impl<M> From<&rustdds::no_key::DeserializedCacheChange<M>> for MessageInfo {
  fn from(dcc: &rustdds::no_key::DeserializedCacheChange<M>) -> MessageInfo {
    MessageInfo {
      received_timestamp: Timestamp::ZERO, // TODO!
      source_timestamp: dcc.source_timestamp(),
      sequence_number: dcc.sequence_number,
      publisher: dcc.writer_guid(),
      related_sample_identity: dcc.related_sample_identity(),
    }
  }
}
