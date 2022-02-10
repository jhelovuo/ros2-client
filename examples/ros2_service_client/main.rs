use mio::{Events, Poll, PollOpt, Ready, Token};
use ros2_client::{
    interfaces::{AddTwoIntsRequest, AddTwoIntsService},
    Context, Node, NodeOptions, ServiceMappings,
};
use rustdds::{policy, Duration, QosPolicies, QosPolicyBuilder};

fn main() {
    pretty_env_logger::init();

    println!(">>> ros2_service starting...");
    let mut node = create_node();
    let service_qos = create_qos();

    println!(">>> ros2_service node started");

    let mut client = node
        .create_client::<AddTwoIntsService>(
            ServiceMappings::Enhanced,
            "/ros2_test_service_add",
            service_qos.clone(),
        )
        .unwrap();

    println!(">>> ros2_service client created");

    let poll = Poll::new().unwrap();

    poll.register(&client, Token(7), Ready::readable(), PollOpt::edge())
        .unwrap();

    println!(">>> request sending...");
    match client.send_request(AddTwoIntsRequest { a: 0, b: 1 }) {
        Ok(id) => {
            println!(">>> request sent {id:?}");
        }
        Err(e) => {
            println!(">>> request sending error {e:?}");
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
                    while let Ok(Some((id, response))) = client.receive_response() {
                        println!(">>> Response received - id: {id:?}, response: {response:?}");
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
    let node = context
        .new_node(
            "rustdds_client",
            "/rustdds",
            NodeOptions::new().enable_rosout(true),
        )
        .unwrap();
    node
}
