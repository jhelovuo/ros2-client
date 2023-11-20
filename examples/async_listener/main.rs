use futures::{future, StreamExt};
use ros2_client::*;

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      "rustdds_listener",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let chatter_topic = node
    .create_topic(
      "/topic",
      String::from("std_msgs::msg::dds_::String_"),
      &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
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

  rosout!(node, ros2::LogLevel::Info, "wow  very listening   such topics  much subscribe.");

  smol::block_on(async {
    subscription_stream
      .for_each(|_result| future::ready(()))
      .await
  });
}
