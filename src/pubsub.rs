use mio::Evented;
use mio::Poll;
use mio::Token;
use mio::Ready;
use mio::PollOpt;

use rustdds::*;

use serde::{Serialize, de::DeserializeOwned};

use std::io;

/// A ROS2 Publisher
///
/// Corresponds to a simplified [`DataWriter`](rustdds::no_key::DataWriter)in DDS
pub struct Publisher<M:Serialize> {
	datawriter: no_key::DataWriterCdr<M>,
}


impl<M:Serialize> Publisher<M> {
	// These must be created from Node
	pub(crate) fn new(datawriter: no_key::DataWriterCdr<M>) -> Publisher<M> {
		Publisher{ datawriter }
	}

	pub fn publish(&self, message:M) -> dds::Result<()> {
		self.datawriter.write(message,None)
	}

	pub fn assert_liveliness(&self) -> dds::Result<()> {
		self.datawriter.assert_liveliness()
	}

	pub fn guid(&self) -> rustdds::GUID {
		self.datawriter.guid()
	}
}
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------
// ----------------------------------------------------

/// A ROS2 Subscription
///
/// Corresponds to a (simplified) [`DataReader`](rustdds::no_key::DataReader) in DDS
pub struct Subscription<M:DeserializeOwned> {
	datareader: no_key::DataReaderCdr<M>
}

impl<M:'static + DeserializeOwned> Subscription<M> {
	// These must be created from Node
	pub(crate) fn new(datareader: no_key::DataReaderCdr<M>) -> Subscription<M> {
		Subscription{ datareader }
	}

	pub fn take(&mut self) -> dds::Result<Option<(M,MessageInfo)>> {
		let ds : Option<no_key::DataSample<M>> = self.datareader.take_next_sample()?;
		Ok(ds.map(|ds| { 
			let mi = MessageInfo::from(ds.sample_info());
			(ds.into_value(),mi)
		}))
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
    self
      .datareader
      .register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self
      .datareader
      .reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.datareader.deregister(poll)
  }
}


#[derive(Copy,Debug,Clone,)]
pub struct MessageInfo {
	pub writer_guid : GUID,
	pub sequence_number: i64,
} // TODO

impl From<&SampleInfo> for MessageInfo {
	fn from(sample_info:&SampleInfo) -> MessageInfo {
		MessageInfo{
			writer_guid: sample_info.publication_handle,
			sequence_number: 0xdeadbeef, // TODO: implement
		}
	}
} 
