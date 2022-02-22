use std::time::{Duration, Instant,};

use mio::{Events, Poll, PollOpt, Ready, Token};

use ros2_client::{
    interfaces::{AddTwoIntsRequest, AddTwoIntsService},
    Context, Node, NodeOptions, ServiceMappings,
};

use rustdds::{policy, QosPolicies, QosPolicyBuilder};

const RESPONSE_TOKEN : Token = Token(7); // Just an arbitrary value

fn main() {
    pretty_env_logger::init();

    println!(">>> ros2_service starting...");
    let mut node = create_node();
    let service_qos = create_qos();

    println!(">>> ros2_service node started");

    let mut client = node
        .create_client::<AddTwoIntsService>(
            ServiceMappings::Enhanced,
            "/add_two_ints",
            service_qos.clone(),
        )
        .unwrap();

    println!(">>> ros2_service client created");

    let poll = Poll::new().unwrap();

    poll.register(&client, RESPONSE_TOKEN, Ready::readable(), PollOpt::edge())
        .unwrap();

    let mut request_generator = 0;
    let mut request_sent_at = Instant::now(); // request rate limiter

    loop {
      //println!(">>> event loop iter");
      let mut events = Events::with_capacity(100);
      poll.poll(&mut events, Some(Duration::from_secs(1))).unwrap();

      for event in events.iter() {
        //println!(">>> New event");
        match event.token() {
          RESPONSE_TOKEN => {
              while let Ok(Some((id, response))) = client.receive_response() {
                  println!(">>> Response received: response: {:?} - response id: {:?}, ",
                           response, id,);
              }
          }
          _ => println!(">>> Unknown poll token {:?}", event.token()),
        }
      }
      let now = Instant::now();
      if now.duration_since(request_sent_at) > Duration::from_secs(2) {
        request_sent_at = now;
        println!(">>> request sending...");
        request_generator += 3;
        let a = request_generator % 5;
        let b = request_generator % 7;
        match client.send_request(AddTwoIntsRequest { a, b }) {
            Ok(id) => {
                println!(">>> request sent a={} b={}, {:?}",a,b,id);
            }
            Err(e) => {
                println!(">>> request sending error {:?}",e);
            }
        }
      }

    }
}

fn create_qos() -> QosPolicies {
    let service_qos: QosPolicies = {
        QosPolicyBuilder::new()
            .reliability(policy::Reliability::Reliable {
                max_blocking_time: rustdds::Duration::from_millis(100),
            })
            .history(policy::History::KeepLast { depth: 1 })
            .build()
    };
    service_qos
}

fn create_node() -> Node {
    let context = Context::new().unwrap();
    let node = context
        .new_node(
            "rustdds_client",
            "/rustdds",
            NodeOptions::new().enable_rosout(true),
        )
        .unwrap();
    node
}
