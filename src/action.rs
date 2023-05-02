use std::{
  marker::PhantomData,
};

use rustdds::{*};
use crate::{action_msgs, unique_identifier_msgs, builtin_interfaces,};

use serde::{Deserialize, Serialize};

use crate::{
  message::Message,
  service::{AService, Client, Server, request_id::RmwRequestId,},
  Subscription, Publisher,
};

pub use action_msgs::{CancelGoalRequest, CancelGoalResponse, GoalStatusEnum, GoalId, GoalInfo, };

use builtin_interfaces::Time;


// A trait to define an Action type
pub trait ActionTypes {
  type GoalType: Message + Clone; // Used by client to set a goal for the server
  type ResultType: Message + Clone; // Used by server to report result when action ends
  type FeedbackType: Message; // Used by server to report progrss during action excution

  fn goal_type_name(&self) -> &str;
  fn result_type_name(&self) -> &str;
  fn feedback_type_name(&self) -> &str;  
}


// This is used to construct an ActionType implementation.
pub struct Action<G,R,F>
{
  g : PhantomData<G>,
  r : PhantomData<R>,
  f : PhantomData<F>,
  goal_typename : String,
  result_typename : String,
  feedback_typename : String,
}

impl<G,R,F> Action<G,R,F> 
where
  G : Message + Clone,
  R : Message + Clone,
  F : Message,
{
  pub fn new(goal_typename: String, result_typename: String, feedback_typename: String) -> Self {
    Self {
      goal_typename, 
      result_typename,
      feedback_typename,
      g : PhantomData, 
      r : PhantomData, 
      f: PhantomData,
    }
  }
}

impl<G,R,F> ActionTypes for Action<G,R,F>
where
  G : Message + Clone,
  R : Message + Clone,
  F : Message,
{
  type GoalType = G;
  type ResultType = R;
  type FeedbackType = F;

  fn goal_type_name(&self) -> &str {
    &self.goal_typename
  }

  fn result_type_name(&self) -> &str {
    &self.result_typename
  }

  fn feedback_type_name(&self) -> &str {
    &self.feedback_typename
  }
}


//TODO: Make fields private, add constructr and accessors.
pub struct ActionClientQosPolicies {
  pub goal_service: QosPolicies,
  pub result_service: QosPolicies,
  pub cancel_service: QosPolicies,
  pub feedback_subscription: QosPolicies,
  pub status_subscription: QosPolicies,
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SendGoalRequest<G> 
{
  pub goal_id : GoalId,
  pub goal : G,
}
impl<G:Message> Message for SendGoalRequest<G> {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SendGoalResponse {
  pub accepted: bool,
  pub stamp: builtin_interfaces::Time,
}
impl Message for SendGoalResponse {}

/// Emulating ROS2 IDL code generator: Result getting service
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GetResultRequest {
  pub goal_id : GoalId,
}
impl Message for GetResultRequest {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GetResultResponse<R> {
  pub status: GoalStatusEnum, // interpretation same as in GoalStatus message?
  pub result: R,
}
impl<R:Message> Message for GetResultResponse<R> {}

/// Emulating ROS2 IDL code generator: Feedback Topic message type
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FeedbackMessage<F> {
  pub goal_id : GoalId,
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

  pub(crate) my_cancel_client: 
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

  pub fn goal_client(&mut self) -> &mut Client<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >> {
    &mut self.my_goal_client
  }
  pub fn cancel_client(&mut self) -> &mut Client<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>> {
    &mut self.my_cancel_client
  }
  pub fn result_client(&mut self) -> &mut Client<AService<GetResultRequest, GetResultResponse<A::ResultType> >> {
    &mut self.my_result_client
  }
  pub fn feedback_subscription(&mut self) -> &mut Subscription< FeedbackMessage<A::FeedbackType> > {
    &mut self.my_feedback_subscription
  }
  pub fn status_subscription(&mut self) -> &mut Subscription<action_msgs::GoalStatusArray> {
    &mut self.my_status_subscription
  }

  /// Returns and id of the Request and id for the Goal.
  /// Request id can be used to recognize correct response from Action Server.
  /// Goal id is later used to communicate Goal status and result.
  pub fn send_goal(&mut self, goal: A::GoalType) -> dds::Result<(RmwRequestId,GoalId)> {
    let goal_id = unique_identifier_msgs::UUID::new_random(); 
    self.my_goal_client.send_request(SendGoalRequest{ goal_id: goal_id.clone(), goal })
      .map( |req_id| (req_id, goal_id))
  }

  /// Receive a response for the specified goal request, or None if response is not yet
  /// available
  pub fn receive_goal_response(&mut self, req_id: RmwRequestId) 
    -> dds::Result<Option<SendGoalResponse>> 
  {
    loop {
      match self.my_goal_client.receive_response() {
        Err(e) => break Err(e),
        Ok(None) => break Ok(None), // not yet
        Ok(Some( (incoming_req_id, resp) )) if incoming_req_id == req_id =>
          break Ok(Some(resp)), // received expected answer
        Ok(Some( _ )) =>  // got someone else's answer. Try again.
          (),
      }
    }
    // We loop here to drain all the answers received so far.
    // The mio .poll() only does not trigger again for the next item, if it has been
    // received already.
  }

  // From ROS2 docs:
  // https://docs.ros2.org/foxy/api/action_msgs/srv/CancelGoal.html
  //
  // Cancel one or more goals with the following policy:
  // - If the goal ID is zero and timestamp is zero, cancel all goals.
  // - If the goal ID is zero and timestamp is not zero, cancel all goals accepted   at or before the timestamp.
  // - If the goal ID is not zero and timestamp is zero, cancel the goal with the given ID regardless of the time it was accepted.
  // - If the goal ID is not zero and timestamp is not zero, cancel the goal with the given ID and all goals accepted at or before the timestamp.

  fn cancel_goal_raw(&mut self, goal_id: GoalId, timestamp: Time) -> dds::Result<RmwRequestId> 
  {
    let goal_info = GoalInfo{ goal_id , stamp: timestamp };
    self.my_cancel_client.send_request( CancelGoalRequest{ goal_info} )
  }

  pub fn cancel_goal(&mut self, goal_id: GoalId) -> dds::Result<RmwRequestId> {
    self.cancel_goal_raw(goal_id, Time::ZERO)
  }

  pub fn cancel_all_goals_before(&mut self, timestamp: Time) -> dds::Result<RmwRequestId> {
    self.cancel_goal_raw(GoalId::ZERO, timestamp)
  }

  pub fn cancel_all_goals(&mut self,) -> dds::Result<RmwRequestId> {
    self.cancel_goal_raw(GoalId::ZERO, Time::ZERO)
  }

  // TODO: The result type is ugly C++ typing. Rewrite to a proper enum, possibly
  // in a higher-level library.
  pub fn receive_cancel_response(&mut self, cancel_request_id: RmwRequestId) 
    -> dds::Result<Option<CancelGoalResponse>> 
  { 
    loop {
      match self.my_cancel_client.receive_response() {
        Err(e) => break Err(e),
        Ok(None) => break Ok(None), // not yet
        Ok(Some( (incoming_req_id, resp ) )) 
          if incoming_req_id == cancel_request_id =>
            break Ok(Some( resp )), // received expected answer
        Ok(Some( _ )) =>  // got someone else's answer. Try again.
          (),
      }
    }    
  }

  pub fn request_result(&mut self, goal_id: GoalId) -> dds::Result<RmwRequestId> {
    self.my_result_client.send_request( GetResultRequest{ goal_id } )
  }

  pub fn receive_result(&mut self, result_request_id: RmwRequestId) 
    -> dds::Result<Option<(GoalStatusEnum, A::ResultType)>> 
  {
    loop {
      match self.my_result_client.receive_response() {
        Err(e) => break Err(e),
        Ok(None) => break Ok(None), // not yet
        Ok(Some( (incoming_req_id, GetResultResponse{ status, result } ) )) 
          if incoming_req_id == result_request_id =>
            break Ok(Some( ( status,result) )), // received expected answer
        Ok(Some( _ )) =>  // got someone else's answer. Try again.
          (),
      }
    }    
  } 


  pub fn receive_feedback(&mut self, goal_id: GoalId) -> dds::Result<Option<A::FeedbackType>> 
  where
    <A as ActionTypes>::FeedbackType: 'static,
  {
    loop {
      match self.my_feedback_subscription.take() {
        Err(e) => break Err(e),
        Ok(None) => break Ok(None),
        Ok(Some( (fb_msg, _msg_info) )) if fb_msg.goal_id == goal_id =>
          break Ok(Some(fb_msg.feedback)),
        Ok(Some(_)) => (), // feedback on some other goal
      }
    }
  }

  /// Note: This does not take GoalId and will therefore report status of all Goals.
  pub fn receive_status(&mut self,) -> dds::Result<Option<action_msgs::GoalStatusArray>> {
    self.my_status_subscription.take()
      .map( |r| r.map( |(gsa,_msg_info)| gsa ) )
  }

} // impl




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
  my_goal_server: 
    Server<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >>,

  my_cancel_server: 
    Server<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>>,

  my_result_server:
    Server<AService<GetResultRequest, GetResultResponse<A::ResultType> >>,

  my_feedback_publisher: Publisher< FeedbackMessage<A::FeedbackType> >,

  my_status_publisher: Publisher<action_msgs::GoalStatusArray>, 

  my_action_name: String,
}


impl<A> ActionServer<A> 
where
  A: ActionTypes,
  A::GoalType: Message + Clone,
  A::ResultType: Message + Clone,
  A::FeedbackType: Message,
{

  pub fn name(&self) -> &str {
    &self.my_action_name
  }

  pub fn goal_server(&mut self) -> &mut Server<AService< SendGoalRequest<A::GoalType>, SendGoalResponse >> {
    &mut self.my_goal_server
  }
  pub fn cancel_server(&mut self) -> &mut Server<AService<action_msgs::CancelGoalRequest, action_msgs::CancelGoalResponse>> {
    &mut self.my_cancel_server
  }
  pub fn result_server(&mut self) -> &mut Server<AService<GetResultRequest, GetResultResponse<A::ResultType> >> {
    &mut self.my_result_server
  }
  pub fn feedback_publisher(&mut self) -> &mut Publisher< FeedbackMessage<A::FeedbackType> > {
    &mut self.my_feedback_publisher
  }
  pub fn my_status_publisher(&mut self) -> &mut Publisher<action_msgs::GoalStatusArray> {
    &mut self.my_status_publisher
  }

  /// Receive a new goal, if available.
  pub fn receive_goal(&mut self) -> dds::Result<Option<(RmwRequestId,SendGoalRequest<A::GoalType>)>> {
    self.my_goal_server.receive_request()
  }

  /// Send a response for the specified goal request
  pub fn send_goal_response(&mut self, req_id: RmwRequestId, resp: SendGoalResponse) 
    -> dds::Result<()> 
  {
    self.my_goal_server.send_response(req_id, resp)
  }


  /// Receive a cancel request, if available.
  pub fn receive_cancel_request(&mut self) -> dds::Result<Option<(RmwRequestId,action_msgs::CancelGoalRequest)>> {
    self.my_cancel_server.receive_request()
  }

  // Respond to a received cancel request
  pub fn send_cancel_response(&mut self, req_id: RmwRequestId, resp: action_msgs::CancelGoalResponse) 
    -> dds::Result<()> 
  {
    self.my_cancel_server.send_response(req_id, resp)
  }



  pub fn receive_result_request(&mut self) -> dds::Result<Option<(RmwRequestId, GetResultRequest)>> {
    self.my_result_server.receive_request()
  }

  pub fn send_result(&mut self, result_request_id: RmwRequestId, resp: GetResultResponse<A::ResultType>) 
    -> dds::Result<()> 
  {
    self.my_result_server.send_response(result_request_id, resp)
  } 


  pub fn send_feedback(&mut self, goal_id: GoalId, feedback: A::FeedbackType) -> dds::Result<()> 
  {
    self.my_feedback_publisher.publish(FeedbackMessage{ goal_id , feedback })
  }

  // Send the status of all known goals.
  pub fn send_goal_statuses(&mut self, goal_statuses: action_msgs::GoalStatusArray) -> dds::Result<()> {
    self.my_status_publisher.publish(goal_statuses)
  }

} // impl

