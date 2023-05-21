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
//
// Run this example and start a client example program from ROS2.




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
  // Use e.g.
  // % export RUST_LOG=warn,ros2_client=debug,minimal_action_server=debug
  // to see what is going on.

  // Set Ctrl-C handler
  let (stop_sender, stop_receiver) = smol::channel::bounded(2);
  ctrlc::set_handler(move || {
    // We will send two stop commands, one for reader, the other for writer.
    stop_sender.send_blocking(()).unwrap_or(());
  })
    .expect("Error setting Ctrl-C handler");
  println!("Press Ctrl-C to quit.");

  let mut node = create_node();

  let service_qos: QosPolicies = {
    QosPolicyBuilder::new()
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: rustdds::Duration::from_millis(100),
      })
      .history(policy::History::KeepLast { depth: 1 })
      .build()
  };

  let publisher_qos : QosPolicies = {
    QosPolicyBuilder::new()
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: rustdds::Duration::from_millis(100),
      })
      .history(policy::History::KeepLast { depth: 1 })
      .durability(policy::Durability::TransientLocal)
      .build()
  };

  let fibonacci_action_qos = action::ActionServerQosPolicies {
    goal_service: service_qos.clone(),
    result_service: service_qos.clone(),
    cancel_service: service_qos.clone(),
    feedback_publisher: publisher_qos.clone(),
    status_publisher: publisher_qos.clone(),
  };

  let mut fibonacci_action_server = action::AsyncActionServer::new(node
    .create_action_server::<FibonacciAction>(
      ServiceMapping::Enhanced,
      "fibonacci",
      &MessageTypeName::new("example_interfaces", "Fibonacci"), 
      fibonacci_action_qos,
    )
    .unwrap());

  let loop_rate = Duration::from_secs(1);

  let main_loop = async {
    let mut run = true;
    let mut stop = stop_receiver.recv().fuse();

    info!("Entering main loop");
    while run {
      futures::select! {
        _ = stop => run = false,

        new_goal_handle = fibonacci_action_server.receive_new_goal().fuse() => {
          match new_goal_handle {
            Err(e) => println!("Goal receive failed: {:?}",e),
            Ok(new_goal_handle) => {
              let fib_order = usize::try_from( *fibonacci_action_server.get_new_goal(new_goal_handle).unwrap()).unwrap();
              info!("New goal. order={fib_order}");
              if  fib_order < 1 || fib_order > 25 {
                fibonacci_action_server.reject_goal(new_goal_handle).await.unwrap();
              } else {
                // goal seems fine, let's go
                let accepted_goal = 
                  fibonacci_action_server.accept_goal(new_goal_handle).await.unwrap();
                info!("Goal accepted. order={fib_order}");
                let executing_goal =
                  fibonacci_action_server.start_executing_goal(accepted_goal).await.unwrap();

                let mut goal_canceled = false;
                let mut fib = Vec::with_capacity( fib_order );
                fib.push(0); // F_0
                fib.push(1); // F_1
                for i in 2..=fib_order {

                  // Update sequence
                  fib.push( fib[i-2] + fib[i-1]);
                  // Publish feedback
                  fibonacci_action_server.publish_feedback(executing_goal, fib.clone()).await.unwrap();
                  info!("Publish feedback");

                  // add some delay, also check for causes to stop
                  futures::select! {
                    _ = stop => {run = false; break }
                    _ = smol::Timer::interval(loop_rate).fuse() => (),
                    cancel_handle = fibonacci_action_server.receive_cancel_request().fuse() => {
                      let cancel_handle = cancel_handle.unwrap();
                      let my_goal = executing_goal.goal_id();
                      if cancel_handle.contains_goal(&my_goal) {
                        info!("Got cancel request!");
                        fibonacci_action_server
                          .respond_to_cancel_requests(&cancel_handle, std::iter::once(my_goal))
                          .await
                          .unwrap();
                        goal_canceled = true;
                        break
                      } else {
                        info!("Received a cancel request for some other goals.");
                      }
                    }
                  }
                } // for

                if goal_canceled {
                  // We need to return a result in case of cancel too.
                  fibonacci_action_server
                    .send_result_response(executing_goal, action::GoalEndStatus::Canceled, fib)
                    .await.unwrap();
                  info!("Goal termianted because of cancel.");
                } else {
                  // goal complete
                  fibonacci_action_server
                    .send_result_response(executing_goal, action::GoalEndStatus::Succeeded, fib)
                    .await.unwrap();
                  info!("Goal succeeded. order={fib_order}");
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

fn create_node() -> Node {
  let context = Context::new().unwrap();
  let node = context
    .new_node(
      "minimal_action_server",
      "/rustdds",
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();
  node
}
