use std::time::Duration;
use chrono::{DateTime, Utc};

//use futures::StreamExt;
use ros2_client::*;

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      NodeName::new("", "time_listener").unwrap(),
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  smol::spawn(node.spinner().unwrap().spin()).detach();

  smol::block_on(async move {
    loop {
      smol::Timer::after(Duration::from_secs(1)).await;
      println!("{:?}", DateTime::<Utc>::from( node.time_now() ) );
    }
  });
}
