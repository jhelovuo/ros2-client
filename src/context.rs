use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

#[cfg(feature = "security")]
use std::path::{PathBuf, Path};


#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use mio::Evented;
use serde::{de::DeserializeOwned, Serialize};
use rustdds::{
  dds::CreateResult,
  no_key::{DeserializerAdapter, SerializerAdapter},
  policy::*,
  *,
};

use crate::{
  builtin_topics,
  gid::Gid,
  node::{Node, NodeOptions},
  node_entities_info::NodeEntitiesInfo,
  participant_entities_info::ParticipantEntitiesInfo,
  pubsub::{Publisher, Subscription},
};

lazy_static! {
  pub static ref DEFAULT_SUBSCRIPTION_QOS: QosPolicies = QosPolicyBuilder::new()
    .durability(Durability::Volatile) // default per table in DDS Spec v1.4 Section 2.2.3 Supported QoS
    .deadline(Deadline(Duration::INFINITE)) // default per table in DDS Spec v1.4 Section 2.2.3 Supported QoS
    .ownership(Ownership::Shared) // default per table in DDS Spec v1.4 Section 2.2.3 Supported QoS
    .reliability(Reliability::BestEffort) // default for DataReaders and Topics
    .history(History::KeepLast { depth: 1 }) // default per table in DDS Spec v1.4 Section 2.2.3 Supported QoS
    .lifespan(Lifespan {
      // default per table in DDS Spec v1.4 Section 2.2.3 Supported QoS
      duration: Duration::INFINITE
    })
    .build();
}

lazy_static! {
  pub static ref DEFAULT_PUBLISHER_QOS: QosPolicies = QosPolicyBuilder::new()
    .durability(Durability::Volatile)
    .deadline(Deadline(Duration::INFINITE))
    .ownership(Ownership::Shared)
    .reliability(Reliability::Reliable{max_blocking_time: Duration::from_millis(100)}) 
      // Reliability = Reliable is the default for DataWriters, different from above.
    .history(History::KeepLast { depth: 1 })
    .lifespan(Lifespan {
      duration: Duration::INFINITE
    })
    .build();
}

#[cfg(feature = "security")]
struct SecurityConfig {
  /// Path to a directory of configuration files.
  security_config_dir: PathBuf,
  /// Password used for decryption the private key file.
  private_key_password: String,
}


pub struct ContextOptions {
  domain_id: u16,
  #[cfg(feature = "security")]
  security_config: Option<SecurityConfig>,
}

impl ContextOptions {
  pub fn new() -> Self {
    Self {
      domain_id: 0,
      #[cfg(feature = "security")]
      security_config: None,
    }
  }

  /// Set the DDS Domain Id.
  ///
  /// Please refer to the 
  /// [ROS_DOMAIN_ID](https://docs.ros.org/en/iron/Concepts/Intermediate/About-Domain-ID.html)
  /// or DDS documentation.
  pub fn domain_id(mut self, domain_id: u16) -> Self {
    self.domain_id = domain_id;
    self
  }

  /// Enable DDS security features.
  ///
  /// Using security requires providing appropriate configuration files.
  ///
  /// [Security in RustDDS](https://github.com/jhelovuo/RustDDS/blob/master/SECURITY.md)
  #[cfg(feature = "security")]
  pub fn enable_security(mut self, security_config_dir: impl AsRef<Path>, 
    private_key_password: String) -> Self 
  {
    self.security_config = 
      Some(SecurityConfig{ 
        security_config_dir: security_config_dir.as_ref().to_path_buf() , 
        private_key_password });
    self
  }

}

impl Default for ContextOptions {
  fn default() -> Self {
    Self::new()
  }
}


/// [Context] communicates with other
/// participants information in ROS2 network. It keeps track of
/// [`NodeEntitiesInfo`]s. Also acts as a wrapper for a RustDDS instance.
///
/// Context is shut down by dropping it, and all of its RosNodes.
/// There should be no need for `ok()` or `shutdown()` methods.
#[derive(Clone)]
pub struct Context {
  inner: Arc<Mutex<ContextInner>>,
}

impl Context {
  /// Create a new Context with default settings.
  pub fn new() -> CreateResult<Context> {
    Self::from_domain_participant(DomainParticipant::new(0)?)
  }

  /// Create a new Context.
  pub fn with_options(opt: ContextOptions) -> CreateResult<Context> {
    #[allow(unused_mut)] // only mutated with security
    let mut dpb =  DomainParticipantBuilder::new( opt.domain_id );

    #[cfg(feature = "security")]
    {
      if let Some(sc) = opt.security_config {
        dpb = 
          dpb.builtin_security( 
            DomainParticipantSecurityConfigFiles::with_ros_default_names(
              sc.security_config_dir,
              sc.private_key_password,
            ) 
          );
      }
    }

    Self::from_domain_participant(dpb.build()?)
  }   

  /// Create a new Context from an existing [`DomainParticipant`].
  pub fn from_domain_participant(domain_participant: DomainParticipant) -> CreateResult<Context> {
    let i = ContextInner::from_domain_participant(domain_participant)?;
    Ok(Context {
      inner: Arc::new(Mutex::new(i)),
    })
  }

  /// Create a new ROS2 [`Node`]
  pub fn new_node(&self, name: &str, namespace: &str, options: NodeOptions) -> CreateResult<Node> {
    Node::new(name, namespace, options, self.clone())
  }

  /// Query which DDS Domain Id we are using.
  pub fn domain_id(&self) -> u16 {
    self.inner.lock().unwrap().domain_participant.domain_id()
  }

  /// Which topics have been discovered?
  pub fn discovered_topics(&self) -> Vec<rustdds::discovery::DiscoveredTopicData> {
    self.domain_participant().discovered_topics()
  }


  pub fn get_all_discovered_external_ros_node_infos(&self) -> HashMap<Gid, Vec<NodeEntitiesInfo>> {
    self.inner.lock().unwrap().external_nodes.clone()
  }

  pub fn get_all_discovered_local_ros_node_infos(&self) -> HashMap<String, NodeEntitiesInfo> {
    self.inner.lock().unwrap().nodes.clone()
  }

  /// Gets our current participant info we have sent to ROS2 network
  pub fn get_ros_participant_info(&self) -> ParticipantEntitiesInfo {
    self.inner.lock().unwrap().get_ros_participant_info()
  }

  /// Get a (handle to) the ROSOut logging Topic.
  ///
  pub fn get_parameter_events_topic(&self) -> Topic {
    self
      .inner
      .lock()
      .unwrap()
      .ros_parameter_events_topic
      .clone()
  }

  /// Get a (handle to) the ROSOut logging Topic.
  ///
  /// Note: The recommended way to write log messages to ROSOut is via the
  /// [`rosout`] macro in module [`node`].
  pub fn get_rosout_topic(&self) -> Topic {
    self.inner.lock().unwrap().ros_rosout_topic.clone()
  }

  /// Get the contained DDS [`DomainParticipant`]. 
  ///
  /// The return value is owned, but it is just a smart pointer.
  ///
  pub fn domain_participant(&self) -> DomainParticipant {
    self.inner.lock().unwrap().domain_participant.clone()
  }

  // -----------------------------------------------------------------------

  pub(crate) fn create_publisher<M>(
    &self,
    topic: &Topic,
    qos: Option<QosPolicies>,
  ) -> dds::CreateResult<Publisher<M>>
  where
    M: Serialize,
  {
    let datawriter = self
      .get_ros_default_publisher()
      .create_datawriter_no_key(topic, qos)?;

    Ok(Publisher::new(datawriter))
  }

  pub(crate) fn create_subscription<M>(
    &self,
    topic: &Topic,
    qos: Option<QosPolicies>,
  ) -> dds::CreateResult<Subscription<M>>
  where
    M: 'static + DeserializeOwned,
  {
    let datareader = self
      .get_ros_default_subscriber()
      .create_simple_datareader_no_key(topic, qos)?;
    Ok(Subscription::new(datareader))
  }

  pub(crate) fn create_datawriter<M, SA>(
    &self,
    topic: &Topic,
    qos: Option<QosPolicies>,
  ) -> dds::CreateResult<no_key::DataWriter<M, SA>>
  where
    SA: SerializerAdapter<M>,
  {
    self
      .get_ros_default_publisher()
      .create_datawriter_no_key(topic, qos)
  }

  pub(crate) fn create_simpledatareader<M, DA>(
    &self,
    topic: &Topic,
    qos: Option<QosPolicies>,
  ) -> dds::CreateResult<no_key::SimpleDataReader<M, DA>>
  where
    M: 'static,
    DA: 'static + DeserializerAdapter<M>,
  {
    self
      .get_ros_default_subscriber()
      .create_simple_datareader_no_key(topic, qos)
  }

  // pub(crate) fn handle_node_read(&mut self) -> Vec<ParticipantEntitiesInfo> {
  //   self.inner.lock().unwrap().handle_node_read()
  // }

  // // Clears all nodes and updates our ContextInfo to ROS2 network
  // pub(crate) fn clear(&mut self) {
  //   self.inner.lock().unwrap().clear();
  // }


  pub(crate) fn add_node_info(&mut self, node_info: NodeEntitiesInfo) {
    self.inner.lock().unwrap().add_node_info(node_info);
  }

  // pub(crate) fn remove_node_info(&mut self, node_info: &NodeEntitiesInfo) {
  //   self.inner.lock().unwrap().remove_node_info(node_info);
  // }

  fn get_ros_default_publisher(&self) -> rustdds::Publisher {
    self.inner.lock().unwrap().ros_default_publisher.clone()
  }

  fn get_ros_default_subscriber(&self) -> rustdds::Subscriber {
    self.inner.lock().unwrap().ros_default_subscriber.clone()
  }

}

struct ContextInner {
  nodes: HashMap<String, NodeEntitiesInfo>,
  external_nodes: HashMap<Gid, Vec<NodeEntitiesInfo>>,
  node_reader: no_key::DataReaderCdr<ParticipantEntitiesInfo>,
  node_writer: no_key::DataWriterCdr<ParticipantEntitiesInfo>,

  domain_participant: DomainParticipant,
  #[allow(dead_code)]
  ros_discovery_topic: Topic,
  ros_default_publisher: rustdds::Publisher,
  ros_default_subscriber: rustdds::Subscriber,

  ros_parameter_events_topic: Topic,
  ros_rosout_topic: Topic,
}

impl ContextInner {
  // "new"
  pub fn from_domain_participant(
    domain_participant: DomainParticipant,
  ) -> CreateResult<ContextInner> {
    let ros_discovery_topic = domain_participant.create_topic(
      builtin_topics::ros_discovery::TOPIC_NAME.to_string(),
      builtin_topics::ros_discovery::TYPE_NAME.to_string(),
      &builtin_topics::ros_discovery::QOS_PUB,
      TopicKind::NoKey,
    )?;

    let ros_default_publisher = domain_participant.create_publisher(&DEFAULT_PUBLISHER_QOS)?;
    let ros_default_subscriber = domain_participant.create_subscriber(&DEFAULT_SUBSCRIPTION_QOS)?;

    let ros_parameter_events_topic = domain_participant.create_topic(
      builtin_topics::parameter_events::TOPIC_NAME.to_string(),
      builtin_topics::parameter_events::TYPE_NAME.to_string(),
      &builtin_topics::parameter_events::QOS,
      TopicKind::NoKey,
    )?;

    let ros_rosout_topic = domain_participant.create_topic(
      builtin_topics::rosout::TOPIC_NAME.to_string(),
      builtin_topics::rosout::TYPE_NAME.to_string(),
      &builtin_topics::rosout::QOS,
      TopicKind::NoKey,
    )?;

    let node_reader =
      ros_default_subscriber.create_datareader_no_key(&ros_discovery_topic, None)?;

    let node_writer = ros_default_publisher.create_datawriter_no_key(&ros_discovery_topic, None)?;

    Ok(ContextInner {
      nodes: HashMap::new(),
      external_nodes: HashMap::new(),
      node_reader,
      node_writer,

      domain_participant,
      ros_discovery_topic,
      ros_default_publisher,
      ros_default_subscriber,
      ros_parameter_events_topic,
      ros_rosout_topic,
    })
  }

  /// Gets our current participant info we have sent to ROS2 network
  pub fn get_ros_participant_info(&self) -> ParticipantEntitiesInfo {
    ParticipantEntitiesInfo::new(
      Gid::from_guid(self.domain_participant.guid()),
      self.nodes.values().cloned().collect(),
    )
  }

  // Adds new NodeEntitiesInfo and updates our ContextInfo to ROS2 network
  fn add_node_info(&mut self, mut node_info: NodeEntitiesInfo) {
    node_info.add_reader(Gid::from_guid(self.node_reader.guid()));
    node_info.add_writer(Gid::from_guid(self.node_writer.guid()));

    self.nodes.insert(node_info.get_full_name(), node_info);
    self.broadcast_node_infos();
  }

  // /// Removes NodeEntitiesInfo and updates our ContextInfo to ROS2 network
  // fn remove_node_info(&mut self, node_info: &NodeEntitiesInfo) {
  //   self.nodes.remove(&node_info.get_full_name());
  //   self.broadcast_node_infos();
  // }

  // /// Clears all nodes and updates our ContextInfo to ROS2 network
  // pub fn clear(&mut self) {
  //   if !self.nodes.is_empty() {
  //     self.nodes.clear();
  //     self.broadcast_node_infos();
  //   }
  // }

  fn broadcast_node_infos(&self) {
    match self
      .node_writer
      .write(self.get_ros_participant_info(), None)
    {
      Ok(_) => (),
      Err(e) => error!("Failed to write into node_writer {:?}", e),
    }
  }

  // /// Fetches all unread ParticipantEntitiesInfos we have received
  // pub fn handle_node_read(&mut self) -> Vec<ParticipantEntitiesInfo> {
  //   let mut pts = Vec::new();
  //   while let Ok(Some(sample)) = self.node_reader.take_next_sample() {
  //     let rpi = sample.value();
  //     match self.external_nodes.get_mut(&rpi.guid()) {
  //       Some(rpi2) => {
  //         *rpi2 = rpi.nodes().clone();
  //       }
  //       None => {
  //         self.external_nodes.insert(rpi.guid(), rpi.nodes().clone());
  //       }
  //     };
  //     pts.push(rpi.clone());
  //   }
  //   pts
  // }

}

impl Evented for Context {
  fn register(
    &self,
    poll: &mio::Poll,
    token: mio::Token,
    interest: mio::Ready,
    opts: mio::PollOpt,
  ) -> std::io::Result<()> {
    poll.register(
      &self.inner.lock().unwrap().node_reader,
      token,
      interest,
      opts,
    )
  }

  fn reregister(
    &self,
    poll: &mio::Poll,
    token: mio::Token,
    interest: mio::Ready,
    opts: mio::PollOpt,
  ) -> std::io::Result<()> {
    poll.reregister(
      &self.inner.lock().unwrap().node_reader,
      token,
      interest,
      opts,
    )
  }

  fn deregister(&self, poll: &mio::Poll) -> std::io::Result<()> {
    poll.deregister(&self.inner.lock().unwrap().node_reader)
  }
}
