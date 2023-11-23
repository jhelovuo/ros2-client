use ros2_client::{Context, Node, NodeOptions};
use async_io::Timer;
fn main() {
  // Here is a fixed path, so this example must be started from
  // RustDDS main directory
  log4rs::init_file("examples/async_talker/log4rs.yaml", Default::default()).unwrap();

  let mut node = create_node();

  let chatter_topic = node
    .create_topic(
      "/topic",
      String::from("std_msgs::msg::dds_::String_"),
      &ros2_client::DEFAULT_PUBLISHER_QOS,
    )
    .unwrap();
  let chatter_publisher = node
    .create_publisher::<String>(&chatter_topic, None)
    .unwrap();
  let mut count = 0;

  let filler: String =
    "All work and no play makes ROS a dull boy. All play and no work makes RTPS a mere toy. "
      .repeat(2);
  smol::block_on(async {
    loop {
      count += 1;
      let message = format!("count={} {}", count, filler);
      println!("Talking, count={} len={}", count, message.len());
      let _ = chatter_publisher.async_publish(message).await;
      Timer::after(std::time::Duration::from_secs(2)).await;
    }
  });
}

fn create_node() -> Node {
  let context = Context::new().unwrap();
  context
    .new_node(
      "rustdds_talker",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap()
}
