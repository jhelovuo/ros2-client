use futures::StreamExt;
use ros2_client::{Context, NodeName, NodeOptions};

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      NodeName::new("/rustdds", "discovery_listener").unwrap(),
      NodeOptions::default(),
    )
    .unwrap();

  let status_event_stream = node.status_receiver().for_each(|event| async move {
    println!("{:?}", event);
  });

  smol::spawn(node.spinner().spin()).detach();

  smol::block_on(status_event_stream);
}
