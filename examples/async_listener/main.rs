use futures::StreamExt;
use ros2_client::*;

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      NodeName::new("/rustdds", "rustdds_listener").unwrap(),
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let chatter_topic = node
    .create_topic(
      &Name::new("/", "topic").unwrap(),
      MessageTypeName::new("std_msgs", "String"),
      &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
    )
    .unwrap();
  let chatter_subscription = node
    .create_subscription::<String>(&chatter_topic, None)
    .unwrap();

  let subscription_stream = chatter_subscription
    .async_stream()
    .for_each(|result| async {
      match result {
        Ok((msg, _)) => println!("I heard: {msg}"),
        Err(e) => eprintln!("Receive request error: {:?}", e),
      }
    });

  // Since we enabled rosout, let's log something
  rosout!(
    node,
    ros2::LogLevel::Info,
    "wow. very listening. such topics. much subscribe."
  );

  smol::block_on(subscription_stream);
}
