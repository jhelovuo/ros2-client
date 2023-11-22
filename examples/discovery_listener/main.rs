use futures::{join, StreamExt};
use log::error;
use ros2_client::{Context, NodeOptions};

pub fn main() {
  let context = Context::new().unwrap();
  let node = context
    .new_node(
      "discovery_listener",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let status_event_stream = node.status_receiver().for_each(|event| async move {
    println!("{:?}", event);
  });

  smol::block_on(async {
    join!(
      // spin worker task
      async { node.spin().await.unwrap_or_else(|e| error!("{e:?}")) },
      // subsribe and print discovery events produced by spinner
      status_event_stream
    )
  });
}
