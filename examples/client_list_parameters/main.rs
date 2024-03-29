use std::env;

use mio::{Events, Poll, PollOpt, Ready, Token};
use ros2_client::{
  interfaces::{ListParametersRequest, ListParametersResponse},
  AService, Context, Name, Node, NodeName, NodeOptions, ServiceMapping, ServiceTypeName,
};
use rustdds::{policy, Duration, QosPolicies, QosPolicyBuilder};

fn main() {
  pretty_env_logger::init();

  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    println!("There is no args");
    return;
  }

  println!(">>> ros2_service starting...");
  let mut node = create_node();
  let service_qos = create_qos();

  println!(">>> ros2_service node started");

  let client = node
    .create_client::<AService<ListParametersRequest, ListParametersResponse>>(
      ServiceMapping::Enhanced,
      &Name::parse(&args[1]).unwrap(),
      &ServiceTypeName::new("rcl_interfaces", "ListParameters"),
      service_qos.clone(),
      service_qos,
    )
    .unwrap();

  println!(">>> ros2_service client created");

  let poll = Poll::new().unwrap();

  poll
    .register(&client, Token(7), Ready::readable(), PollOpt::edge())
    .unwrap();

  println!(">>> request sending...");
  let request = ListParametersRequest {
    DEPTH_RECURSIVE: 0,
    depth: 0,
    prefixes: vec![],
  };

  match client.send_request(request) {
    Ok(id) => {
      println!(">>> request sent {:?}", id);
    }
    Err(e) => {
      println!(">>> request sending error {:?}", e);
    }
  }

  'e_loop: loop {
    println!(">>> event loop iter");
    let mut events = Events::with_capacity(100);
    poll.poll(&mut events, None).unwrap();

    for event in events.iter() {
      println!(">>> New event");
      match event.token() {
        Token(7) => {
          if let Ok(Some((id, response))) = client.receive_response() {
            println!(
              ">>> Response received -  response: {:?}, id: {:?},",
              response, id,
            );
            break 'e_loop;
          }
        }
        _ => println!(">>> Unknown poll token {:?}", event.token()),
      }
    }
  }
}

fn create_qos() -> QosPolicies {
  let service_qos: QosPolicies = {
    QosPolicyBuilder::new()
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: Duration::from_millis(100),
      })
      .history(policy::History::KeepLast { depth: 1 })
      .build()
  };
  service_qos
}

fn create_node() -> Node {
  let context = Context::new().unwrap();
  context
    .new_node(
      NodeName::new("/rustdds", "rustdds_client").unwrap(),
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap()
}
