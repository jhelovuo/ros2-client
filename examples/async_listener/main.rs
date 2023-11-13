use futures::{future, StreamExt};
use ros2_client::{Context, Node, NodeOptions};
use rustdds::{
  policy::{self, Deadline, Lifespan},
  Duration, QosPolicies, QosPolicyBuilder,
};
pub fn main() {
  let mut node = create_node();
  let topic_qos = create_qos();

  let chatter_topic = node
    .create_topic(
      "/topic",
      String::from("std_msgs::msg::dds_::String_"),
      &topic_qos,
    )
    .unwrap();
  let chatter_subscription = node
    .create_subscription::<String>(&chatter_topic, None)
    .unwrap();
  let subscription_stream = chatter_subscription.async_stream().then(|result| async {
    match result {
      Ok((msg, _)) => println!("I heard: {msg}"),
      Err(e) => eprintln!("Receive request error: {:?}", e),
    }
  });
  smol::block_on(async {
    subscription_stream
      .for_each(|_result| future::ready(()))
      .await
  });
}

fn create_qos() -> QosPolicies {
  let service_qos: QosPolicies = {
    QosPolicyBuilder::new()
      .history(policy::History::KeepLast { depth: 10 })
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: Duration::from_millis(100),
      })
      .durability(policy::Durability::Volatile)
      .deadline(Deadline(Duration::INFINITE))
      .lifespan(Lifespan {
        duration: Duration::INFINITE,
      })
      .liveliness(policy::Liveliness::Automatic {
        lease_duration: Duration::INFINITE,
      })
      .build()
  };
  service_qos
}

fn create_node() -> Node {
  let context = Context::new().unwrap();
  context
    .new_node(
      "rustdds_listener",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap()
}
