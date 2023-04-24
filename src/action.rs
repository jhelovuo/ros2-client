use rustdds::{*};
use crate::{action_msgs, unique_identifier_msgs, builtin_interfaces,};

use serde::{Deserialize, Serialize};

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
  pub(crate) goal_service: QosPolicies,
  pub(crate) result_service: QosPolicies,
  pub(crate) cancel_service: QosPolicies,
  pub(crate) feedback_subscription: QosPolicies,
  pub(crate) status_subscription: QosPolicies,
}

#[allow(dead_code)] // TODO: Use this
pub struct ActionServerQosPolicies {
  pub(crate) goal_service: QosPolicies,
  pub(crate) result_service: QosPolicies,
  pub(crate) cancel_service: QosPolicies,
  pub(crate) feedback_publication: QosPolicies,
  pub(crate) status_publication: QosPolicies,
}


/// Emulating ROS2 IDL code generator: Goal sending/setting service
#[derive(Clone, Serialize, Deserialize)]
pub struct SendGoalRequest<G> 
{
  pub goal_id : unique_identifier_msgs::UUID,
  pub goal : G,
}
impl<G:Message> Message for SendGoalRequest<G> {}

#[derive(Clone, Serialize, Deserialize)]
pub struct SendGoalResponse {
  pub accepted: bool,
  pub stamp: builtin_interfaces::Time,
}
impl Message for SendGoalResponse {}

/// Emulating ROS2 IDL code generator: Result getting service
#[derive(Clone, Serialize, Deserialize)]
pub struct GetResultRequest {
  pub goal_id : unique_identifier_msgs::UUID,
}
impl Message for GetResultRequest {}

#[derive(Clone, Serialize, Deserialize)]
pub struct GetResultResponse<R> {
  pub status: i8, // interpretation same as in GoalStatus message?
  pub result: R,
}
impl<R:Message> Message for GetResultResponse<R> {}

/// Emulating ROS2 IDL code generator: Feedback Topic message type
#[derive(Clone, Serialize, Deserialize)]
pub struct FeedbackMessage<F> {
  pub goal_id : unique_identifier_msgs::UUID,
  pub feedback: F,
}
impl<F:Message> Message for FeedbackMessage<F> {}

pub struct ActionClient<A> 
where 
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{
  pub(crate) my_goal_client: 
    Client<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >>,

  pub(crate)my_cancel_client: 
    Client<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>>,

  pub(crate) my_result_client:
    Client<AService<GetResultRequest, GetResultResponse<A::ResultType> >>,

  pub(crate) my_feedback_subscription: Subscription< FeedbackMessage<A::FeedbackType> >,

  pub(crate) my_status_subscription: Subscription<action_msgs::GoalStatusArray>, 

  pub(crate) my_action_name: String,
}

impl<A> ActionClient<A> 
where
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{

  pub fn name(&self) -> &str {
    &self.my_action_name
  }

  pub fn goal_client(&self) -> &Client<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >> {
    &self.my_goal_client
  }
  pub fn cancel_client(&self) -> &Client<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>> {
    &self.my_cancel_client
  }
  pub fn result_client(&self) -> &Client<AService<GetResultRequest, GetResultResponse<A::ResultType> >> {
    &self.my_result_client
  }
  pub fn feedback_subscription(&self) -> &Subscription< FeedbackMessage<A::FeedbackType> > {
    &self.my_feedback_subscription
  }
  pub fn status_subscription(&self) -> &Subscription<action_msgs::GoalStatusArray> {
    &self.my_status_subscription
  }

}




// Example topic names and types at DDS level:

// rq/turtle1/rotate_absolute/_action/send_goalRequest : turtlesim::action::dds_::RotateAbsolute_SendGoal_Request_
// rr/turtle1/rotate_absolute/_action/send_goalReply : turtlesim::action::dds_::RotateAbsolute_SendGoal_Response_

// rq/turtle1/rotate_absolute/_action/cancel_goalRequest  : action_msgs::srv::dds_::CancelGoal_Request_
// rr/turtle1/rotate_absolute/_action/cancel_goalReply  : action_msgs::srv::dds_::CancelGoal_Response_

// rq/turtle1/rotate_absolute/_action/get_resultRequest : turtlesim::action::dds_::RotateAbsolute_GetResult_Request_
// rr/turtle1/rotate_absolute/_action/get_resultReply : turtlesim::action::dds_::RotateAbsolute_GetResult_Response_

// rt/turtle1/rotate_absolute/_action/feedback : turtlesim::action::dds_::RotateAbsolute_FeedbackMessage_

// rt/turtle1/rotate_absolute/_action/status : action_msgs::msg::dds_::GoalStatusArray_

#[allow(dead_code)] // TODO: Use this
pub struct ActionServer<A> 
where 
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{
  goal_server: 
    Server<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >>,

  cancel_server: 
    Server<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>>,

  result_server:
    Server<AService<GetResultRequest, GetResultResponse<A::ResultType> >>,

  feedback_publisher: Publisher< FeedbackMessage<A::FeedbackType> >,

  status_publisher: Publisher<action_msgs::GoalStatusArray>, 

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


