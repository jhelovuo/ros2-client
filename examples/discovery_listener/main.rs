use futures::StreamExt;
use ros2_client::{Context, NodeOptions};

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      "discovery_listener",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let status_event_stream = node.status_receiver().for_each(|event| async move {
    println!("{:?}", event);
  });

  smol::spawn(node.spinner().spin()).detach();

  smol::block_on(status_event_stream);
}
