use log::error;
use mio::{Events, Poll, PollOpt, Ready, Token};
use ros2_client::{Context, Message, Node, NodeOptions, Service, ServiceMappings};
use rustdds::{policy, Duration, QosPolicies, QosPolicyBuilder};
use serde::{Deserialize, Serialize};

fn main() {
    println!("ros2_service starting...");
    let mut node = create_node();
    let service_qos = create_qos();
    let default_qos = QosPolicies::default();

    println!("ros2_service node started");

    let mut server = node
        .create_server::<AddTwoIntsService>(
            ServiceMappings::Enhanced,
            "/ros2_test_service",
            default_qos.clone(),
        )
        .unwrap();

    println!("ros2_service server created");

    let poll = Poll::new().unwrap();

    poll.register(&server, Token(1), Ready::all(), PollOpt::edge())
        .unwrap();

    loop {
        println!("event loop iter");
        let mut events = Events::with_capacity(100);
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            println!("New event");
            match event.token() {
                Token(1) => {
                    while let Ok(Some((id, request))) = server.receive_request() {
                        println!("Request received - id: {id:?}, request: {request:?}");
                        let response = AddTwoIntsResponse { sum: 99 };
                        match server.send_response(id, response.clone()) {
                            Ok(_) => {
                                println!(
                                    "Server response send for id: {id:?}, response: {response:?}"
                                )
                            }
                            Err(e) => {
                                error!("Server response error: {e}");
                            }
                        }
                    }
                }
                _ => println!("Unknown poll token {:?}", event.token()),
            }
        }
    }
}

pub struct AddTwoIntsService {}

impl Service for AddTwoIntsService {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
    fn request_type_name() -> String {
        "example_interfaces::srv::dds_::AddTwoInts_Request_".to_owned()
    }
    fn response_type_name() -> String {
        "example_interfaces::srv::dds_::AddTwoInts_Response_".to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTwoIntsRequest {
    pub a: i64,
    pub b: i64,
}
impl Message for AddTwoIntsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTwoIntsResponse {
    pub sum: i64,
}
impl Message for AddTwoIntsResponse {}

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
            "rustdds_server",
            "/rustdds",
            NodeOptions::new().enable_rosout(true),
        )
        .unwrap();
    node
}
