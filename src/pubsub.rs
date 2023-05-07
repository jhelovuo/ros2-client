use std::io;

use mio::{Evented, Poll, PollOpt, Ready, Token};
use futures::{pin_mut, stream::StreamExt};
use rustdds::{rpc::SampleIdentity, *};
use serde::{de::DeserializeOwned, Serialize};

//use crate::action::*;

/// A ROS2 Publisher
///
/// Corresponds to a simplified [`DataWriter`](rustdds::no_key::DataWriter)in
/// DDS
pub struct Publisher<M: Serialize> {
  datawriter: no_key::DataWriterCdr<M>,
}

impl<M: Serialize> Publisher<M> {
  // These must be created from Node
  pub(crate) fn new(datawriter: no_key::DataWriterCdr<M>) -> Publisher<M> {
    Publisher { datawriter }
  }

  pub fn publish(&self, message: M) -> dds::Result<()> {
    self.datawriter.write(message, Some(Timestamp::now()))
  }

  // pub(crate) fn publish_with_options(
  //   &self,
  //   message: M,
  //   wo: WriteOptions,
  // ) -> dds::Result<rustdds::rpc::SampleIdentity> {
  //   self.datawriter.write_with_options(message, wo)
  // }

  pub fn assert_liveliness(&self) -> dds::Result<()> {
    self.datawriter.assert_liveliness()
  }

  pub fn guid(&self) -> rustdds::GUID {
    self.datawriter.guid()
  }

  pub async fn async_publish(&self, message: M) -> dds::Result<()> {
    self
      .datawriter
      .async_write(message, Some(Timestamp::now()))
      .await
  }

  #[allow(dead_code)] // This is for async Service implementation. Remove this when it is implemented.
  pub(crate) async fn async_publish_with_options(
    &self,
    message: M,
    wo: WriteOptions,
  ) -> dds::Result<rustdds::rpc::SampleIdentity> {
    self.datawriter.async_write_with_options(message, wo).await
  }
}
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------

/// A ROS2 Subscription
///
/// Corresponds to a (simplified) [`DataReader`](rustdds::no_key::DataReader) in
/// DDS
pub struct Subscription<M: DeserializeOwned> {
  datareader: no_key::SimpleDataReaderCdr<M>,
}

impl<M: 'static + DeserializeOwned> Subscription<M> {
  // These must be created from Node
  pub(crate) fn new(datareader: no_key::SimpleDataReaderCdr<M>) -> Subscription<M> {
    Subscription { datareader }
  }

  pub fn take(&self) -> dds::Result<Option<(M, MessageInfo)>> {
    self.datareader.drain_read_notifications();
    let ds: Option<no_key::DeserializedCacheChange<M>> = self.datareader.try_take_one()?;
    Ok(ds.map(|ds| {
      let mi = MessageInfo::from(&ds);
      (ds.into_value(), mi)
    }))
  }

  pub async fn async_take(&self) -> dds::Result<(M, MessageInfo)> {
    let async_stream = self.datareader.as_async_stream();
    pin_mut!(async_stream);
    let (item, _stream) = async_stream.into_future().await;
    match item {
      Some(Err(e)) => Err(e),
      Some(Ok(ds)) => Ok({
        let mi = MessageInfo::from(&ds);
        (ds.into_value(), mi)
      }),
      None => unimplemented!(), // This should be safe, because DataReader stream cannot end.
    }
  }

  pub fn guid(&self) -> rustdds::GUID {
    self.datareader.guid()
  }
}

impl<D> Evented for Subscription<D>
where
  D: DeserializeOwned,
{
  // We just delegate all the operations to datareader, since it
  // already implements Evented
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.datareader.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self.datareader.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.datareader.deregister(poll)
  }
}

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
