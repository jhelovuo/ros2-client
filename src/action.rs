use rustdds::{*};
use crate::action_msgs;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
  message::Message,
  service::{AService, Client, Server,},
  Subscription, Publisher,
};

pub trait ActionTypes {
  type GoalType: Message + Clone; // Used by client to set a goal for the server
  type ResultType: Message + Clone; // Used by server to report result when action ends
  type FeedbackType: Message; // Used by server to report progrss during action excution

  fn goal_type_name() -> String;
  fn result_type_name() -> String;
  fn feedback_type_name() -> String;  
}


pub struct ActionClientQosPolicies {
  goal_service_qos: QosPolicies,
  result_service_qos: QosPolicies,
  cancel_service_qos: QosPolicies,
  feedback_subscription_qos: QosPolicies,
  status_subscription_qos: QosPolicies,
}

pub struct ActionServerQosPolicies {
  goal_service_qos: QosPolicies,
  result_service_qos: QosPolicies,
  cancel_service_qos: QosPolicies,
  feedback_publication_qos: QosPolicies,
  status_publication_qos: QosPolicies,
}

#[derive(Clone, Serialize, Deserialize)]
struct GoalResponse {} // placeholder - how is this defined??
impl Message for GoalResponse {}

#[derive(Clone, Serialize, Deserialize)]
struct ResultRequest {} // placeholder - how is this defined??
impl Message for ResultRequest {}



pub struct ActionClient<A> 
where 
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{
  goal_client: 
    Client<AService< A::GoalType, GoalResponse >>,

  cancel_client: 
    Client<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>>,

  result_client:
    Client<AService<ResultRequest, A::ResultType>>,

  feedback_subscription: Subscription<A::FeedbackType>, 
  status_subscription: Subscription<action_msgs::GoalStatusArray>, 

  action_name: String,
}

impl<A> ActionClient<A> 
where
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{

}

// Example topic names and types at DDS level:

// rq/turtle1/rotate_absolute/_action/send_goalRequest : turtlesim::action::dds_::RotateAbsolute_SendGoal_Request_
// rr/turtle1/rotate_absolute/_action/send_goalReply : turtlesim::action::dds_::RotateAbsolute_SendGoal_Response_

// rq/turtle1/rotate_absolute/_action/cancel_goalRequest  : action_msgs::srv::dds_::CancelGoal_Request_
// rr/turtle1/rotate_absolute/_action/cancel_goalReply  : action_msgs::srv::dds_::CancelGoal_Response_

// rq/turtle1/rotate_absolute/_action/get_resultRequest : turtlesim::action::dds_::RotateAbsolute_GetResult_Request_
// rr/turtle1/rotate_absolute/_action/get_resultReply : turtlesim::action::dds_::RotateAbsolute_GetResult_Response_

// rt/turtle1/rotate_absolute/_action/feedback : turtlesim::action::dds_::RotateAbsolute_FeedbackMessage_

// rt/turtle1/rotate_absolute/_action/status : action_msgs::srv::dds_::GoalStatusArray_




// pub struct ActionServer<A> 
// where A: ActionTypes,
// {
//   a: Phant
// }

// impl<A> ActionServer<A> {

// }
