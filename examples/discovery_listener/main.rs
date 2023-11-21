use futures::{join, StreamExt};
use log::error;
use ros2_client::{Context, NodeOptions};

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
  let subscription_stream = chatter_subscription
    .async_stream()
    .for_each(|result| async {
      match result {
        Ok((msg, _)) => println!("I heard: {msg}"),
        Err(e) => eprintln!("Receive request error: {:?}", e),
      }
    });

  let status_event_stream = node.status_receiver().for_each(|event| async move {
    println!("{:?}", event);
  });

  smol::block_on(async {
    join!(
      // actual data subscription
      subscription_stream,
      // spin worker task
      async { node.spin().await.unwrap_or_else(|e| error!("{e:?}")) },
      // subsribe and print discovery events produced by spinner
      status_event_stream
    )
  });
}
