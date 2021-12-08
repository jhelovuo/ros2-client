#[allow(unused_imports)]
use rustdds::{
  dds::qos::{
    policy::{
      Deadline, DestinationOrder, Durability, History, LatencyBudget, Lifespan, Liveliness,
      Ownership, Reliability,
    },
    QosPolicies, QosPolicyBuilder,
  },
  dds::data_types::DDSDuration as Duration,
};



pub mod ros_discovery {
  use super::*;

  lazy_static! {
    pub static ref QOS: QosPolicies = 
      QosPolicyBuilder::new()
        .durability(Durability::TransientLocal)
        .deadline(Deadline(Duration::DURATION_INFINITE))
        .ownership(Ownership::Shared)
        .reliability(Reliability::Reliable { max_blocking_time: Duration::DURATION_ZERO })
        .history(History::KeepLast { depth: 1 })
        .lifespan(Lifespan {duration: Duration::DURATION_INFINITE})
        .build();
  }

  pub const TOPIC_NAME: &'static str = "ros_discovery_info";

  pub const TYPE_NAME: &'static str = "rmw_dds_common::msg::dds_::ParticipantEntitiesInfo_";
}

pub mod parameter_events {
  use super::*;

  lazy_static! {
    pub static ref QOS: QosPolicies = 
      QosPolicyBuilder::new()
        .durability(Durability::TransientLocal)
        .reliability(Reliability::Reliable { max_blocking_time: Duration::DURATION_ZERO })
        .history(History::KeepLast { depth: 1 })
        .build();
  }

  pub const TOPIC_NAME: &'static str = "rt/parameter_events";

  pub const TYPE_NAME: &'static str = "rcl_interfaces::msg::dds_::ParameterEvent_";
}

pub mod rosout {
  use super::*;

  lazy_static! {
    pub static ref QOS: QosPolicies = 
      QosPolicyBuilder::new()
        .durability(Durability::TransientLocal)
        .deadline(Deadline(Duration::DURATION_INFINITE))
        .ownership(Ownership::Shared)
        .reliability(Reliability::Reliable { max_blocking_time: Duration::DURATION_ZERO })
        .history(History::KeepLast { depth: 1 })
        .lifespan(Lifespan {duration: Duration::from_secs(10)})
        .build();
  }

  pub const TOPIC_NAME: &'static str = "rt/rosout";

  pub const TYPE_NAME: &'static str = "rcl_interfaces::msg::dds_::Log_";
}
