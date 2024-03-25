use std::time::Duration;
use std::convert::TryInto;
use chrono::{DateTime, Utc};

//use futures::StreamExt;
use ros2_client::*;

pub fn main() {
  let context = Context::new().unwrap();
  let mut node = context
    .new_node(
      NodeName::new("", "time_braodcaster").unwrap(),
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let clock_publisher = node.create_publisher::<builtin_interfaces::Time>( 
    &node.create_topic(
      &Name::new("","clock").unwrap(), 
      MessageTypeName::new("builtin_interfaces","Time"),
      &DEFAULT_PUBLISHER_QOS  
    ).unwrap(), 
    None)
  .unwrap();

  smol::spawn(node.spinner().unwrap().spin()).detach();

  // Define at which rates simlated time proceeds vs. real time.
  // Tiacks below are equal in length.
  // This also defines how often simulated clock is updated.
  let sim_time_tick = Duration::from_millis(1000);
  let real_time_tick = Duration::from_millis(250);

  let mut sim_time = node.time_now();

  smol::block_on(async move {
    loop {
      println!("tick {:?}", DateTime::<Utc>::from( sim_time ) );
      clock_publisher.publish(sim_time.into()).unwrap();
      sim_time = sim_time + sim_time_tick.try_into().unwrap() ;
      smol::Timer::after(real_time_tick).await;
    }
  });
}
