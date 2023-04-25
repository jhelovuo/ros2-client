use std::time::Duration;

#[allow(unused_imports)]
use ::log::{debug, error, info, warn};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;
use serde::{Deserialize, Serialize};
use termion::raw::*;
use ros2_client::{ros2, *};
use rustdds::*;
use ui::{RosCommand, UiController};

// modules
mod ui;

const TURTLE_CMD_VEL_READER_TOKEN: Token = Token(1);
const ROS2_COMMAND_TOKEN: Token = Token(2);
const TURTLE_POSE_READER_TOKEN: Token = Token(3);
const RESET_CLIENT_TOKEN: Token = Token(4);
const SET_PEN_CLIENT_TOKEN: Token = Token(5);
const SPAWN_CLIENT_TOKEN: Token = Token(6);
const KILL_CLIENT_TOKEN: Token = Token(7);

// This corresponds to ROS2 message type
// https://github.com/ros2/common_interfaces/blob/master/geometry_msgs/msg/Twist.msg
//
// The struct definition must have a layout corresponding to the
// ROS2 msg definition to get compatible serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Twist {
  pub linear: Vector3,
  pub angular: Vector3,
}

// https://docs.ros2.org/foxy/api/turtlesim/msg/Pose.html
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose {
  pub x: f32,
  pub y: f32,
  pub theta: f32,
  pub linear_velocity: f32,
  pub angular_velocity: f32,
}

// This corresponds to ROS2 message type
// https://github.com/ros2/common_interfaces/blob/master/geometry_msgs/msg/Vector3.msg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

impl Vector3 {
  pub const ZERO: Vector3 = Vector3 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
  };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenRequest {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub width: u8,
  pub off: u8,
}

impl Message for PenRequest {}

fn main() {
  // Here is a fixed path, so this example must be started from
  // RustDDS main directory
  log4rs::init_file("examples/turtle_teleop/log4rs.yaml", Default::default()).unwrap();

  let (command_sender, command_receiver) = mio_channel::sync_channel::<RosCommand>(10);
  let (readback_sender, readback_receiver) = mio_channel::sync_channel(10);
  let (pose_sender, pose_receiver) = mio_channel::sync_channel(10);
  let (message_sender, message_receiver) = mio_channel::sync_channel(2);

  // For some strange reason the ROS2 messaging event loop is in a separate thread
  // and we talk to it using (mio) mpsc channels.
  let jhandle = std::thread::Builder::new()
    .name("ros2_loop".into())
    .spawn(move || {
      ros2_loop(
        command_receiver,
        readback_sender,
        pose_sender,
        message_sender,
      )
    })
    .unwrap();

  // From termion docs:
  // "A terminal restorer, which keeps the previous state of the terminal,
  // and restores it, when dropped.
  // Restoring will entirely bring back the old TTY state."
  // So the point of _stdout_restorer is that it will restore the TTY back to
  // its original cooked mode when the variable is dropped.
  let _stdout_restorer = std::io::stdout().into_raw_mode().unwrap();

  // UI loop, which is in the main thread
  let mut main_control = UiController::new(
    std::io::stdout(),
    command_sender,
    readback_receiver,
    pose_receiver,
    message_receiver,
  );
  main_control.start();

  jhandle.join().unwrap(); // wait until threads exit.

  // need to wait a bit for cleanup, beacuse drop is not waited for join
  std::thread::sleep(Duration::from_millis(10));
}

fn ros2_loop(
  command_receiver: mio_channel::Receiver<RosCommand>,
  readback_sender: mio_channel::SyncSender<Twist>,
  pose_sender: mio_channel::SyncSender<Pose>,
  message_sender: mio_channel::SyncSender<String>,
) {
  info!("ros2_loop");

  let topic_qos: QosPolicies = {
    QosPolicyBuilder::new()
      .durability(policy::Durability::Volatile)
      .liveliness(policy::Liveliness::Automatic {
        lease_duration: ros2::Duration::DURATION_INFINITE,
      })
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: ros2::Duration::from_millis(100),
      })
      .history(policy::History::KeepLast { depth: 10 })
      .build()
  };

  let mut ros_context = Context::new().unwrap();

  let mut ros_node = ros_context
    .new_node(
      "turtle_teleop", // name
      "/ros2_demo",    // namespace
      NodeOptions::new().enable_rosout(true),
    )
    .unwrap();

  let turtle_cmd_vel_topic = ros_node
    .create_topic(
      "/turtle1/cmd_vel",
      String::from("geometry_msgs::msg::dds_::Twist_"),
      &topic_qos,
    )
    .unwrap();

  // The point here is to publish Twist for the turtle
  let turtle_cmd_vel_writer = ros_node
    .create_publisher::<Twist>(&turtle_cmd_vel_topic, None)
    .unwrap();

  // But here is how to read it also, if anyone is interested.
  // This should show what is the turle command in case someone else is
  // also issuing commands, i.e. there are two turtla controllers running.
  let mut turtle_cmd_vel_reader = ros_node
    .create_subscription::<Twist>(&turtle_cmd_vel_topic, None)
    .unwrap();

  let turtle_pose_topic = ros_node
    .create_topic(
      "/turtle1/pose",
      String::from("turtlesim::msg::dds_::Pose_"),
      &topic_qos,
    )
    .unwrap();
  let mut turtle_pose_reader = ros_node
    .create_subscription::<Pose>(&turtle_pose_topic, None)
    .unwrap();

  // Perepare for controlling 2nd turtle
  let turtle2_cmd_vel_topic = ros_node
    .create_topic(
      "/turtle2/cmd_vel",
      String::from("geometry_msgs::msg::dds_::Twist_"),
      &topic_qos,
    )
    .unwrap();
  let turtle_cmd_vel_writer2 = ros_node
    .create_publisher::<Twist>(&turtle2_cmd_vel_topic, None)
    .unwrap();

  // Turtle has services, let's construct some clients.
  
  //pub struct EmptyService {}

  #[derive(Debug, Clone, Serialize, Deserialize)]
  // ROS2 Foxy with eProsima DDS crashes if the EmptyMessage is really empty,
  // so we put in a dummy byte.
  pub struct EmptyMessage {
    dummy: u8,
  }
  impl EmptyMessage {
    pub fn new() -> EmptyMessage {
      EmptyMessage { dummy: 1 }
    }
  }

  impl Message for EmptyMessage {}

  let service_qos: QosPolicies = {
    QosPolicyBuilder::new()
      .reliability(policy::Reliability::Reliable {
        max_blocking_time: ros2::Duration::from_millis(100),
      })
      .history(policy::History::KeepLast { depth: 1 })
      .build()
  };

  // create_client cyclone version tested against ROS2 Galactic. Obviously with
  // CycloneDDS. Seems to work on the same host only.
  //
  // create_client enhanced version tested against
  // * ROS2 Foxy with eProsima DDS. Works to another host also.
  // * ROS2 Galactic with RTI Connext (rmw_connextdds, not rmw_connext_cpp)
  //   Environment variable RMW_CONNEXT_REQUEST_REPLY_MAPPING=extended Works to
  //   another host also.
  //
  // * create_client basic version is untested.
  // Service responses do not fully work yet.
  let empty_srv_name = MessageTypeName::new("std_srvs", "Empty");
  let mut reset_client = ros_node
    .create_client::<AService<EmptyMessage, EmptyMessage>>(
      ServiceMappings::Enhanced,
      "/reset",
      &empty_srv_name.dds_request_type(),
      &empty_srv_name.dds_response_type(),
      service_qos.clone(),
      service_qos.clone(),
    )
    .unwrap();

  // another client

  // from https://docs.ros2.org/foxy/api/turtlesim/srv/SetPen.html
  let set_pen_srv_name = MessageTypeName::new("turtlesim", "SetPen");
  let mut set_pen_client = ros_node
    .create_client::<AService< PenRequest , () >>(
      ServiceMappings::Enhanced,
      "turtle1/set_pen",
      &set_pen_srv_name.dds_request_type(),
      &set_pen_srv_name.dds_response_type(),
      service_qos.clone(),
      service_qos.clone(),
    )
    .unwrap();

  // third client

  // from https://docs.ros2.org/foxy/api/turtlesim/srv/Spawn.html

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct SpawnRequest {
    pub x: f32,
    pub y: f32,
    pub theta: f32,
    pub name: String,
  }
  impl Message for SpawnRequest {}

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct SpawnResponse {
    pub name: String,
  }
  impl Message for SpawnResponse {}

  type SpawnService = AService<SpawnRequest, SpawnResponse>;

  let spawn_srv_name = MessageTypeName::new("turtlesim", "Spawn");
  let mut spawn_client = ros_node
    .create_client::<SpawnService>(
      ServiceMappings::Enhanced,
      "spawn",
      &spawn_srv_name.dds_request_type(),
      &spawn_srv_name.dds_response_type(),
      service_qos.clone(),
      service_qos.clone(),
    )
    .unwrap();


  // kill client

  // from https://docs.ros2.org/foxy/api/turtlesim/srv/Spawn.html
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct KillRequest {
    pub name: String,
  }
  impl Message for KillRequest {}

  type KillService = AService<KillRequest,EmptyMessage>;

  let kill_srv_name = MessageTypeName::new("turtlesim", "Kill");
  let mut kill_client = ros_node
    .create_client::<KillService>(
      ServiceMappings::Enhanced,
      "kill",
      &kill_srv_name.dds_request_type(),
      &kill_srv_name.dds_response_type(),
      service_qos.clone(),
      service_qos.clone(),
    )
    .unwrap();

  // Set up event loop

  let poll = Poll::new().unwrap();

  poll
    .register(
      &command_receiver,
      ROS2_COMMAND_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &turtle_cmd_vel_reader,
      TURTLE_CMD_VEL_READER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();
  poll
    .register(
      &turtle_pose_reader,
      TURTLE_POSE_READER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();
  poll
    .register(
      &reset_client,
      RESET_CLIENT_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();
  poll
    .register(
      &set_pen_client,
      SET_PEN_CLIENT_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &spawn_client,
      SPAWN_CLIENT_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  poll
    .register(
      &kill_client,
      KILL_CLIENT_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();

  info!("Entering event_loop");
  'event_loop: loop {
    let mut events = Events::with_capacity(100);
    poll.poll(&mut events, None).unwrap();

    for event in events.iter() {
      match event.token() {
        ROS2_COMMAND_TOKEN => {
          while let Ok(command) = command_receiver.try_recv() {
            match command {
              RosCommand::StopEventLoop => {
                info!("Stopping main event loop");
                ros_context.clear();
                break 'event_loop;
              }
              RosCommand::TurtleCmdVel { turtle_id, twist } => {
                match match turtle_id {
                  1 => turtle_cmd_vel_writer.publish(twist.clone()),
                  2 => turtle_cmd_vel_writer2.publish(twist.clone()),
                  _ => panic!("WTF?"),
                } {
                  Ok(_) => {
                    info!("Wrote to ROS2 {:?}", twist);
                  }
                  Err(e) => {
                    error!("Failed to write to turtle writer. {:?}", e);
                    ros_node.clear_node();
                    return;
                  }
                }
              }
              RosCommand::Reset => match reset_client.send_request(EmptyMessage::new()) {
                Ok(id) => {
                  info!("Reset request sent {:?}", id);
                }
                Err(e) => {
                  error!("Failed to send request: {:?}", e);
                }
              },
              RosCommand::SetPen(pen_request) => {
                match set_pen_client.send_request(pen_request.clone()) {
                  Ok(id) => {
                    info!("set_pen request sent {:?} {:?}", id, pen_request);
                  }
                  Err(e) => {
                    error!("Failed to send request: {:?}", e);
                  }
                }
              }

              RosCommand::Spawn(name) => {
                match spawn_client.send_request(SpawnRequest {
                  x: 1.0,
                  y: 1.0,
                  theta: 0.0,
                  name,
                }) {
                  Ok(id) => {
                    info!("spawn request sent {:?} ", id);
                  }
                  Err(e) => {
                    error!("Failed to send request: {:?}", e);
                  }
                }
              }
              RosCommand::Kill(name) => {
                match kill_client.send_request(KillRequest { name }) {
                  Ok(id) => {
                    info!("kill request sent {:?} ", id);
                  }
                  Err(e) => {
                    error!("Failed to send request: {:?}", e);
                  }
                }
              }
            }
          }
        }
        TURTLE_CMD_VEL_READER_TOKEN => {
          while let Ok(Some(twist)) = turtle_cmd_vel_reader.take() {
            readback_sender.send(twist.0).unwrap();
          }
        }
        TURTLE_POSE_READER_TOKEN => {
          while let Ok(Some(pose)) = turtle_pose_reader.take() {
            pose_sender.send(pose.0).unwrap();
          }
        }
        RESET_CLIENT_TOKEN => {
          while let Ok(Some(id)) = reset_client.receive_response() {
            message_sender
              .send(format!("Turtle reset acknowledged: {:?}", id))
              .unwrap();
            info!("Turtle reset acknowledged: {:?}", id);
          }
        }
        SET_PEN_CLIENT_TOKEN => {
          while let Ok(Some(id)) = set_pen_client.receive_response() {
            message_sender
              .send(format!("set_pen acknowledged: {:?}", id))
              .unwrap();
            info!("set_pen acknowledged: {:?}", id);
          }
        }
        KILL_CLIENT_TOKEN => {
          while let Ok(Some(id)) = kill_client.receive_response() {
            message_sender
              .send(format!("Turtle kill acknowledged: {:?}", id))
              .unwrap();
            info!("Turtle kill acknowledged: {:?}", id);
          }
        }

        _ => {
          error!("Unknown poll token {:?}", event.token())
        }
      } // match
    } // for
  }
}
