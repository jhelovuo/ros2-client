use std::convert::TryFrom;

use std::time::Duration;

#[allow(unused_imports)]
use log::{debug, error, info, warn};


use futures::{FutureExt as StdFutureExt, };

use ros2_client::{Context, Node, NodeOptions, ServiceMapping, action, 
  MessageTypeName };
use rustdds::{policy, QosPolicies, QosPolicyBuilder};

// Test / demo program of ROS2 Action, server side.
//
// To set up a server from ROS2:
//
// ....
//
// Then run this example.




// Original action definition
// https://docs.ros2.org/latest/api/action_tutorials_interfaces/action/Fibonacci.html
//
// int32 order
// ---
// int32[] sequence
// ---
// int32[] partial_sequence


// Rust version of action type definition
//
// We define the action using standard/primitive types, but we could
// just as well use e.g.
// struct FibonacciActionGoal{ goal: i32 }
// or any other tuple/struct that contains only an i32.
type FibonacciAction = action::Action<i32, Vec<i32>, Vec<i32>>;


fn main() {
  pretty_env_logger::init();

  // Set Ctrl-C handler
  let (stop_sender, stop_receiver) = smol::channel::bounded(2);
  ctrlc::set_handler(move || {
    // We will send two stop commands, one for reader, the other for writer.
    stop_sender.send_blocking(()).unwrap_or(());
  })
    .expect("Error setting Ctrl-C handler");
  println!("Press Ctrl-C to quit.");

  let mut node = create_node();
  let service_qos = create_qos();

  let fibonacci_action_qos = action::ActionServerQosPolicies {
    goal_service: service_qos.clone(),
    result_service: service_qos.clone(),
    cancel_service: service_qos.clone(),
    feedback_publisher: service_qos.clone(),
    status_publisher: service_qos.clone(),
  };

  let mut fibonacci_action_server = action::AsyncActionServer::new(node
    .create_action_server::<FibonacciAction>(
      ServiceMapping::Enhanced,
      "fibonacci",
      &MessageTypeName::new("example_interfaces", "Fibonacci"), 
      fibonacci_action_qos,
    )
    .unwrap());

  let main_loop = async {
    let mut run = true;
    let mut stop = stop_receiver.recv().fuse();

    // let mut tick_stream = // Send new Goal at every tick, if previous one is not running.
    //   futures::StreamExt::fuse(smol::Timer::interval(Duration::from_secs(1)));

    while run {
      futures::select! {
        _ = stop => run = false,

        new_goal_handle = fibonacci_action_server.receive_new_goal().fuse() => {
          match new_goal_handle {
            Err(e) => println!("Goal receive failed: {:?}",e),
            Ok(new_goal_handle) => {
              let fib_order = usize::try_from( *fibonacci_action_server.get_new_goal(new_goal_handle).unwrap()).unwrap();
              if  fib_order < 1 || fib_order > 25 {
                fibonacci_action_server.reject_goal(new_goal_handle).await.unwrap();
              } else {
                // goal seems fine, let's go
                let accepted_goal = 
                  fibonacci_action_server.accept_goal(new_goal_handle).await.unwrap();
                let executing_goal =
                  fibonacci_action_server.start_executing_goal(accepted_goal).await.unwrap();
                let mut fib = Vec::with_capacity( fib_order );
                fib.push(0); // F_0
                fib.push(1); // F_1
                for i in 2..=fib_order {
                  fib.push( fib[i-2] + fib[i-1]);
                  smol::Timer::interval(Duration::from_secs(1)).await; // some computation delay
                  fibonacci_action_server.publish_feedback(executing_goal, fib.clone()).await.unwrap();
                }
              }
            }
          }
        }


      } // select!
    } // while
    debug!("main loop done");
  };

  // run it!
  smol::block_on(main_loop);
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
