use rustdds::*;

use serde::{Serialize, de::DeserializeOwned};

/// A ROS2 Publisher
///
/// Corresponds to a simplified [`DataWriter`](rustdds::no_key::DataWriter)in DDS
pub struct Publisher<M:Serialize> {
	datawriter: no_key::DataWriterCdr<M>,
}


impl<M:Serialize> Publisher<M> {
	// These must be ceated from Node
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
	// These must be ceated from Node
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

#[derive(Copy,Debug,Clone,)]
pub struct MessageInfo {} // TODO

impl From<&SampleInfo> for MessageInfo {
	fn from(_sample_info:&SampleInfo) -> MessageInfo {
		MessageInfo{}
	}
} 
