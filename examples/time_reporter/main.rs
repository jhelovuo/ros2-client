use std::time::Duration;
use chrono::{DateTime, Utc};

use ros2_client::*;

pub fn main() {
  log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      NodeName::new("/", "time_reporter").unwrap(),
      NodeOptions::new()
        .enable_rosout(true),
    )
    .unwrap();

  smol::spawn(node.spinner().unwrap().spin()).detach();

  node.set_parameter("use_sim_time", ParameterValue::Boolean(true)).unwrap();

  let clock_publisher = node.create_publisher::<builtin_interfaces::Time>( 
    &node.create_topic(
      &Name::new("/","clock").unwrap(), 
      MessageTypeName::new("builtin_interfaces","Time"),
      &DEFAULT_PUBLISHER_QOS  
    ).unwrap(), 
    None)
  .unwrap();


  smol::block_on(async move {
    loop {
      smol::Timer::after(Duration::from_secs(1)).await;
      clock_publisher.publish(node.time_now().into()).unwrap();
      println!("{:?}", DateTime::<Utc>::from( node.time_now() ) );
    }
  });
}
