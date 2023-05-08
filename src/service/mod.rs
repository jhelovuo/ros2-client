use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use std::{
  io,
  marker::PhantomData,
  sync::atomic,
};

use mio::{Evented, Poll, PollOpt, Ready, Token};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use futures::{StreamExt, pin_mut};
use bytes::{Bytes,BytesMut,BufMut};

use rustdds::{rpc::*, *, serialization::{deserialize_from_cdr}};


use crate::{
  message::Message,
  node::Node,
  pubsub::{MessageInfo},
};

pub mod request_id;
pub use request_id::*;

// --------------------------------------------
// --------------------------------------------

/// Service trait pairs the Request and Response types together.
/// Additonally, it ensures that Response and Request are Messages
/// (Serializable), and we have a means to name the types.
pub trait Service {
  type Request: Message;
  type Response: Message;
  fn request_type_name(&self) -> &str;
  fn response_type_name(&self) -> &str;
}

// --------------------------------------------
// --------------------------------------------

/// AService is a means of constructing a descriptor for a Service on the fly.
/// This allows generic code to construct a Service from the types of
/// request and response.
pub struct AService<Q, S>
where
  Q: Message,
  S: Message,
{
  q: PhantomData<Q>,
  s: PhantomData<S>,
  req_type_name: String,
  resp_type_name: String,
}

impl<Q, S> AService<Q, S>
where
  Q: Message,
  S: Message,
{
  pub fn new(req_type_name: String, resp_type_name: String) -> Self {
    Self {
      req_type_name,
      resp_type_name,
      q: PhantomData,
      s: PhantomData,
    }
  }
}

impl<Q, S> Service for AService<Q, S>
where
  Q: Message,
  S: Message,
{
  type Request = Q;
  type Response = S;

  fn request_type_name(&self) -> &str {
    &self.req_type_name
  }

  fn response_type_name(&self) -> &str {
    &self.resp_type_name
  }
}

// --------------------------------------------
// --------------------------------------------

/// There are different and incompatible ways to map Services onto DDS Topics.
/// The mapping used by ROS2 depends on the DDS implementation used and its
/// configuration. For details, see OMG Specification
/// [RPC over DDS](https://www.omg.org/spec/DDS-RPC/1.0/About-DDS-RPC/) Section "7.2.4 Basic and Enhanced Service Mapping for RPC over DDS"
/// RPC over DDS" . which defines Service Mappings "Basic" and "Enhanced"
/// ServiceMapping::Cyclone reporesents a third mapping used by RMW for CycloneDDS.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceMapping {
  /// "Basic" service mapping from RPC over DDS specification.
  /// * RTI Connext with `RMW_CONNEXT_REQUEST_REPLY_MAPPING=basic`, but this is
  ///   not tested, so may not work.
  Basic,

  /// "Enhanced" service mapping from RPC over DDS specification.
  /// * ROS2 Foxy with eProsima DDS,
  /// * ROS2 Galactic with RTI Connext (rmw_connextdds, not rmw_connext_cpp) -
  ///   set environment variable `RMW_CONNEXT_REQUEST_REPLY_MAPPING=extended`
  ///   before running ROS2 executable.
  Enhanced,

  /// CycloneDDS-specific service mapping.
  /// Specification for this mapping is unknown, technical details are
  /// reverse-engineered from ROS2 sources.
  /// * ROS2 Galactic with CycloneDDS - Seems to work on the same host only, not
  ///   over actual network.
  Cyclone,
}

// trait Wrapper is for interfacing to Service-specific (De)SerializerAdapter.
// These adapters are essentially pass-through, and do no actual serialization.
// (De)Serialization is done in Wrappers, because they know which ServiceMapping to
// apply, unlike (De)Serializer or their adapters. ServiceMapping must be known in order
// to decode or generate the wire representation.
trait Wrapper {
  fn from_bytes_and_ri(input_bytes: &[u8], encoding: RepresentationIdentifier,) -> Self;
  fn bytes(&self) -> Bytes;
}

#[derive(Serialize, Deserialize)]
struct RequestWrapper<R> {
  serialized_message: Bytes,
  encoding: RepresentationIdentifier,
  phantom: PhantomData<R>,
}

impl<R:Message> Wrapper for RequestWrapper<R> {
  fn from_bytes_and_ri(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Self {
    RequestWrapper {
      serialized_message: Bytes::copy_from_slice(input_bytes), // cloning here
      encoding,
      phantom: PhantomData,
    }
  }
  fn bytes(&self) -> Bytes {
    self.serialized_message.clone()
  }
}

impl<R:Message> RequestWrapper<R> {

  // This will decode the RequestWrapper to Request in Server
  fn unwrap(&self, service_mapping: ServiceMapping, message_info: &MessageInfo) 
    -> dds::Result<(RmwRequestId, R)> 
  {
    match service_mapping {
      ServiceMapping::Basic => {
        // 1. decode "RequestHeader" and
        // 2. decode Request
        let mut bytes = self.serialized_message.clone(); // ref copy only
        let (header,header_size) =
          deserialize_from_cdr::<BasicRequestHeader>(&bytes, self.encoding)?;
        if bytes.len() < header_size {
          dds::Error::serialization_error("Service request too short")
        } else {
          let _header_bytes = bytes.split_off(header_size);
          let (request,_request_bytes) =
            deserialize_from_cdr::<R>(&bytes, self.encoding)?;
          Ok((RmwRequestId::from(header.request_id), request))
        }
      }
      ServiceMapping::Enhanced => {
        // Enhanced mode does not use any header in the DDS payload.
        // Therefore, we use a wrapper that is identical to the payload.
        let (request,_request_bytes) =
            deserialize_from_cdr::<R>(&self.serialized_message, self.encoding)?;
        Ok( (RmwRequestId::from(message_info.sample_identity()), request ) )
      }
      ServiceMapping::Cyclone => {
        cyclone_unwrap::<R>(
          self.serialized_message.clone(), message_info.writer_guid(), self.encoding)
      }   
    }
  }

  // Client creates new RequestWrappers from Requests
  fn new(
    service_mapping: ServiceMapping,
    r_id: RmwRequestId,
    encoding: RepresentationIdentifier,
    request:R ) -> dds::Result<Self> 
  {
    let mut ser_buffer = BytesMut::with_capacity( std::mem::size_of::<R>() * 3 / 2 ).writer();

    // First, write header
    match service_mapping {
      ServiceMapping::Basic => {
        let basic_header = BasicRequestHeader::new( r_id.into() );
        serialization::to_writer_endian(&mut ser_buffer, &basic_header, encoding)?;    
      }
      ServiceMapping::Enhanced => {
        // This mapping does not use any header, so nothing to do here.
      }
      ServiceMapping::Cyclone => {
        let cyclone_header = CycloneHeader::new(r_id);
        serialization::to_writer_endian(&mut ser_buffer, &cyclone_header, encoding)?;    
      }   
    }
    // Second, write request  
    serialization::to_writer_endian(&mut ser_buffer, &request, encoding)?;
    // Ok, assemble result
    Ok(RequestWrapper {
      serialized_message: ser_buffer.into_inner().freeze(),
      encoding,
      phantom: PhantomData,
    })
  }
} 

#[derive(Serialize, Deserialize)]
struct ResponseWrapper<R> {
  serialized_message: Bytes,
  encoding: RepresentationIdentifier,
  rri_for_send: RmwRequestId,
  phantom: PhantomData<R>,
}

impl<R:Message> Wrapper for ResponseWrapper<R> {
  fn from_bytes_and_ri(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Self {
    ResponseWrapper {
      serialized_message: Bytes::copy_from_slice(input_bytes), // cloning here
      encoding,
      rri_for_send: RmwRequestId::default(), // dummy data for client side. Not used.
      phantom: PhantomData,
    }
  }
  fn bytes(&self) -> Bytes {
    self.serialized_message.clone()
  }
}

impl<R:Message> ResponseWrapper<R> {

  // Client decodes ResponseWrapper to Response
  // message_info is from Server's reponse message
  fn unwrap(&self, service_mapping: ServiceMapping, message_info: MessageInfo, client_guid:GUID) 
    -> dds::Result<(RmwRequestId, R)> 
  {
    match service_mapping {
      ServiceMapping::Basic => {
        let mut bytes = self.serialized_message.clone(); // ref copy only
        let (header,header_size) =
          deserialize_from_cdr::<BasicReplyHeader>(&bytes, self.encoding)?;
        if bytes.len() < header_size {
          dds::Error::serialization_error("Service response too short")
        } else {
          let _header_bytes = bytes.split_off(header_size);
          let (response,_bytes) =
            deserialize_from_cdr::<R>(&bytes, self.encoding)?;
          Ok((RmwRequestId::from(header.related_request_id), response))
        }
      }
      ServiceMapping::Enhanced => {
        // Enhanced mode does not use any header in the DDS payload.
        // Therefore, we use a wrapper that is identical to the payload.
        let (response,_response_bytes) =
            deserialize_from_cdr::<R>(&self.serialized_message, self.encoding)?;
        let related_sample_identity = match message_info.related_sample_identity() {
          Some(rsi) => rsi,
          None => {
            return dds::Error::serialization_error("ServiceMapping=Enhanced, but response message did not have related_sample_identity paramter!")
          }
        };
        Ok( (RmwRequestId::from(related_sample_identity), response ) )
      }
      ServiceMapping::Cyclone => {
        // Cyclone constructs the client GUID from two parts
        let mut client_guid_bytes = [0; 16];
        {
          let (first_half, second_half) = client_guid_bytes.split_at_mut(8);

          // This seems a bit odd, but source is
          // https://github.com/ros2/rmw_connextdds/blob/master/rmw_connextdds_common/src/common/rmw_impl.cpp
          // function take_response()
          first_half.copy_from_slice(&client_guid.to_bytes().as_slice()[0..8]);

          // This is received in the wrapper header
          second_half.copy_from_slice(&message_info.writer_guid().to_bytes()[8..16]);
        }
        let client_guid = GUID::from_bytes(client_guid_bytes);

        cyclone_unwrap::<R>(self.serialized_message.clone(), client_guid , self.encoding)
      }   
    }
  }

  // Server creaates new ResponseWrapper from Response
  fn new(
    service_mapping: ServiceMapping,
    r_id: RmwRequestId,
    encoding: RepresentationIdentifier,
    response:R ) -> dds::Result<Self> 
  {
    let mut ser_buffer = BytesMut::with_capacity( std::mem::size_of::<R>() * 3 / 2 ).writer();
    match service_mapping {
      ServiceMapping::Basic => {
        let basic_header = BasicReplyHeader::new( r_id.into() );
        serialization::to_writer_endian(&mut ser_buffer, &basic_header, encoding)?;    
      }
      ServiceMapping::Enhanced => {
        // No header, nothing to write here.
      }
      ServiceMapping::Cyclone => {
        let cyclone_header = CycloneHeader::new(r_id);
        serialization::to_writer_endian(&mut ser_buffer, &cyclone_header, encoding)?;    
      }   
    }
    serialization::to_writer_endian(&mut ser_buffer, &response, encoding)?;
    let serialized_message = ser_buffer.into_inner().freeze();
    Ok(ResponseWrapper {
      serialized_message,
      encoding,
      rri_for_send: r_id,
      phantom: PhantomData,
    })    
  }
} 

// Basic mode header is specified in
// RPC over DDS Section "7.5.1.1.1 Common Types"
#[derive(Serialize, Deserialize)]
pub struct BasicRequestHeader {
  // "struct RequestHeader":
  request_id: SampleIdentity,
  instance_name: String, // This is apparently not used: Always sent as empty string.
}
impl BasicRequestHeader {
  fn new(request_id: SampleIdentity) -> Self {
    BasicRequestHeader {
      request_id, instance_name: "".to_string(),
    }
  }
}
impl Message for BasicRequestHeader {}

#[derive(Serialize, Deserialize)]
pub struct BasicReplyHeader {
  // "struct ReplyHeader":
  related_request_id: SampleIdentity,
  remote_exception_code: u32, /* It is uncertain if this is ever used. Transmitted as zero
                               * ("REMOTE_EX_OK"). */
}
impl BasicReplyHeader {
  fn new(related_request_id: SampleIdentity) -> Self {
    BasicReplyHeader {
      related_request_id, remote_exception_code: 0,
    }
  }
}
impl Message for BasicReplyHeader {}



// Cyclone mode header
//
// This is reverse-engineered from
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/rmw_node.cpp
// https://github.com/ros2/rmw_cyclonedds/blob/master/rmw_cyclonedds_cpp/src/serdata.hpp
// This is a header that Cyclone puts in DDS messages. Same header is used for
// Requst and Response.
#[derive(Serialize, Deserialize)]
pub struct CycloneHeader {
  guid_second_half: [u8; 8], // CycloneDDS RMW only sends last 8 bytes of client GUID
  sequence_number_high: i32,
  sequence_number_low: u32,
}
impl CycloneHeader {
  fn new(r_id: RmwRequestId) -> Self {
    let sn = r_id.sequence_number;
    let mut guid_second_half = [0; 8];
    // writer_guid means client GUID (i.e. request writer)
    guid_second_half.copy_from_slice(&r_id.writer_guid.to_bytes()[8..16]);

    CycloneHeader {
      guid_second_half,
      sequence_number_high: sn.high(),
      sequence_number_low: sn.low(),
    }
  }
}
impl Message for CycloneHeader {}

// helper function, because Cyclone Request and Response unwrapping/decoding are
// the same.
fn cyclone_unwrap<R:Message>(serialized_message:Bytes, writer_guid:GUID, encoding:RepresentationIdentifier) 
  -> dds::Result<(RmwRequestId, R)> 
{
  // 1. decode "CycloneHeader" and
  // 2. decode Request/response
  let mut bytes = serialized_message.clone(); // ref copy only, to make "mutable"
  let (header,header_size) =
    deserialize_from_cdr::<CycloneHeader>(&bytes, encoding)?;
  if bytes.len() < header_size {
    dds::Error::serialization_error("Service message too short")
  } else {
    let _header_bytes = bytes.split_off(header_size);
    let (response,_response_bytes) =
      deserialize_from_cdr::<R>(&bytes, encoding)?;
    let req_id = RmwRequestId {
      writer_guid, // TODO: This seems to be completely wrong!!!
      // When we are the client, we get half of Client GUID on the CycloneHeader, other half from Client State
      // when we are the server, we get half of Client GUID on the CycloneHeader, other half from writer_guid.
      sequence_number: request_id::SequenceNumber::from_high_low(
        header.sequence_number_high,
        header.sequence_number_low
      )
    };
    Ok((req_id, response))
  }
}


type SimpleDataReaderR<RW> = no_key::SimpleDataReader<RW,ServiceDeserializerAdapter<RW>>;
type DataWriterR<RW> = no_key::DataWriter<RW,ServiceSerializerAdapter<RW>>;

struct ServiceDeserializerAdapter<RW> {
  phantom: PhantomData<RW>,
}
struct ServiceSerializerAdapter<RW> {
  phantom: PhantomData<RW>,
}

impl<RW> ServiceDeserializerAdapter<RW> {
  const REPR_IDS: [RepresentationIdentifier; 2] = [
    RepresentationIdentifier::CDR_BE,
    RepresentationIdentifier::CDR_LE,
  ];  
}


impl<RW:Wrapper> no_key::DeserializerAdapter<RW> for ServiceDeserializerAdapter<RW>
where
  RW: DeserializeOwned,
{

  fn supported_encodings() -> &'static [RepresentationIdentifier] {
    &Self::REPR_IDS
  }

  fn from_bytes(input_bytes: &[u8], encoding: RepresentationIdentifier) -> Result<RW, serialization::error::Error> {
    Ok(RW::from_bytes_and_ri(input_bytes, encoding))
  }
}

impl<RW:Wrapper> no_key::SerializerAdapter<RW> for ServiceSerializerAdapter<RW>
where
  RW: Serialize,
{
  fn output_encoding() -> RepresentationIdentifier {
    RepresentationIdentifier::CDR_LE
  }

  fn to_bytes(value: &RW) -> Result<Bytes, serialization::error::Error> {
    Ok(value.bytes())
  }
}



// --------------------------------------------
// --------------------------------------------
/// Server end of a ROS2 Service
pub struct Server<S>
where
  S:Service,
  S::Request: Message,
  S::Response: Message,
{
  service_mapping: ServiceMapping,
  request_receiver: SimpleDataReaderR<RequestWrapper<S::Request>>,
  response_sender: DataWriterR<ResponseWrapper<S::Response>>,
}


impl<S> Server<S>
where
  S: 'static + Service,

{
  pub(crate) fn new(
    service_mapping: ServiceMapping, 
    node: &mut Node,
    request_topic: &Topic,
    response_topic: &Topic,
    qos_request: Option<QosPolicies>,
    qos_response: Option<QosPolicies>,
) -> dds::Result<Self> {
    let request_receiver =
      node.create_simpledatareader
      ::<RequestWrapper<S::Request>, ServiceDeserializerAdapter<RequestWrapper<S::Request>>>(
        request_topic, qos_request)?;
    let response_sender =
      node.create_datawriter
      ::<ResponseWrapper<S::Response>, ServiceSerializerAdapter<ResponseWrapper<S::Response>>>(
        response_topic, qos_response)?;

    debug!(
      "Created new Server: requests={} response={}",
      request_topic.name(),
      response_topic.name()
    );

    Ok(Server::<S>{
      service_mapping,
      request_receiver,
      response_sender,
    })
  }

  pub fn receive_request(&self) -> dds::Result<Option<(RmwRequestId, S::Request)>> {
    self.request_receiver.drain_read_notifications();
    let dcc_rw: Option<no_key::DeserializedCacheChange<RequestWrapper<S::Request>>> 
        = self.request_receiver.try_take_one()?;

    match dcc_rw {
      None => Ok(None),
      Some(dcc) => {
        let mi = MessageInfo::from(&dcc);
        let req_wrapper = dcc.into_value();
        let (ri,req) = req_wrapper.unwrap(self.service_mapping, &mi)?;
        Ok(Some((ri,req)))
      }
    } // match
  }

  pub fn send_response(&self, rmw_req_id: RmwRequestId, response: S::Response) -> dds::Result<()> {
    let resp_wrapper = 
      ResponseWrapper::<S::Response>::new(
        self.service_mapping, 
        rmw_req_id, 
        RepresentationIdentifier::CDR_LE,
        response)?;
    let write_opts = WriteOptionsBuilder::new()
      .source_timestamp(Timestamp::now()) // always add source timestamp
      .related_sample_identity(SampleIdentity::from(rmw_req_id))
      // TODO: Check if this is right. Cyclone mapping does not send 
      // Related Sample Identity in
      // WriteOptions (QoS ParameterList), but within data payload.
      // But maybe it is not harmful to send it in both?
      .build();
    self.response_sender.write_with_options(resp_wrapper, write_opts)
      .map(|_| ())  // lose SampleIdentity result
  }

  pub async fn async_receive_request(&self) -> dds::Result<(RmwRequestId, S::Request)> {
    let dcc_stream = self.request_receiver.as_async_stream();
    pin_mut!(dcc_stream);
    let (dcc_rw, _tail) : (Option<dds::Result<no_key::DeserializedCacheChange<RequestWrapper<S::Request>>>> , _ )= 
      dcc_stream.into_future().await;

    match dcc_rw {
      None => Err(dds::Error::Internal{ reason:
        "SimpleDataReader value stream unexpectedly ended!".to_string() }),
        // This should never occur, because topic do not "end". 
      Some(Err(e)) => Err(e),
      Some(Ok(dcc)) => {
        let mi = MessageInfo::from(&dcc);
        let req_wrapper = dcc.into_value();
        let (ri,req) = req_wrapper
          .unwrap(self.service_mapping, &mi)?;
        Ok((ri,req))
      }
    } // match
  }

  pub async fn async_send_response(&self, rmw_req_id: RmwRequestId, response: S::Response) -> dds::Result<()> {
    let resp_wrapper = 
      ResponseWrapper::<S::Response>::new(
        self.service_mapping, 
        rmw_req_id, 
        RepresentationIdentifier::CDR_LE,
        response)?;
    let write_opts = WriteOptionsBuilder::new()
      .source_timestamp(Timestamp::now()) // always add source timestamp
      .related_sample_identity(SampleIdentity::from(rmw_req_id))
      // TODO: Check if this is right. Cyclone mapping does not send 
      // Related Sample Identity in
      // WriteOptions (QoS ParameterList), but within data payload.
      // But maybe it is not harmful to send it in both?
      .build();
    self.response_sender.async_write_with_options(resp_wrapper, write_opts)
      .await
      .map(|_| ())  // lose SampleIdentity result
  }


}


impl<S> Evented for Server<S>
where
  S: 'static + Service,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.request_receiver.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self.request_receiver.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.request_receiver.deregister(poll)
  }
}

/// Client end of a ROS2 Service
pub struct Client<S> 
where
  S:Service,
  S::Request: Message,
  S::Response: Message,
{
  service_mapping: ServiceMapping,
  request_sender: DataWriterR<RequestWrapper<S::Request>>,
  response_receiver: SimpleDataReaderR<ResponseWrapper<S::Response>>,
  sequence_number_gen: atomic::AtomicI64, // used by basic and cyclone
  client_guid: GUID, // used by the Cyclone ServiceMapping
}


impl<S> Client<S>
where
  S: 'static + Service,
{
  pub(crate) fn new(
    service_mapping: ServiceMapping, 
    node: &mut Node,
    request_topic: &Topic,
    response_topic: &Topic,
    qos_request: Option<QosPolicies>,
    qos_response: Option<QosPolicies>,
  ) -> dds::Result<Self> {
    let request_sender =
      node.create_datawriter
      ::<RequestWrapper<S::Request>, ServiceSerializerAdapter<RequestWrapper<S::Request>>>(
        request_topic, qos_request)?;
    let response_receiver =
      node.create_simpledatareader
      ::<ResponseWrapper<S::Response>, ServiceDeserializerAdapter<ResponseWrapper<S::Response>>>(
        response_topic, qos_response)?;

    debug!(
      "Created new Client: request={} response={}",
      request_topic.name(),
      response_topic.name()
    );
    let client_guid = request_sender.guid();
    Ok(Client::<S>{
      service_mapping,
      request_sender,
      response_receiver,
      sequence_number_gen: atomic::AtomicI64::new(SequenceNumber::default().into()),
      client_guid,
    })
  }

  pub fn send_request(&self, request: S::Request) -> dds::Result<RmwRequestId> {
    self.increment_sequence_number();
    let gen_rmw_req_id = 
      RmwRequestId {
        writer_guid: self.client_guid,
        sequence_number: self.sequence_number(),
      };
    let req_wrapper = 
      RequestWrapper::<S::Request>::new(
        self.service_mapping, 
        gen_rmw_req_id, 
        RepresentationIdentifier::CDR_LE,
        request)?;
    let write_opts_builder = WriteOptionsBuilder::new()
      .source_timestamp(Timestamp::now()); // always add source timestamp

    let write_opts_builder = 
      if self.service_mapping == ServiceMapping::Enhanced {
        write_opts_builder
      } else {
        write_opts_builder.related_sample_identity(SampleIdentity::from(gen_rmw_req_id))
      };
    let sent_rmw_req_id =
      self.request_sender.write_with_options(req_wrapper, write_opts_builder.build())
        .map( RmwRequestId::from )?;

    match self.service_mapping {
      ServiceMapping::Enhanced => Ok(sent_rmw_req_id),
      ServiceMapping::Basic |
      ServiceMapping::Cyclone => Ok(gen_rmw_req_id),
    }  
  }

  pub fn receive_response(&self) -> dds::Result<Option<(RmwRequestId, S::Response)>> {
    self.response_receiver.drain_read_notifications();
    let dcc_rw: Option<no_key::DeserializedCacheChange<ResponseWrapper<S::Response>>> 
        = self.response_receiver.try_take_one()?;

    match dcc_rw {
      None => Ok(None),
      Some(dcc) => {
        let mi = MessageInfo::from(&dcc);
        let res_wrapper = dcc.into_value();
        let (ri,res) = res_wrapper
          .unwrap(self.service_mapping, mi , self.client_guid)?;
        Ok(Some((ri,res)))
      }
    } // match
  }

  pub async fn async_send_request(&self, request: S::Request) -> dds::Result<RmwRequestId> {
    self.increment_sequence_number();
    let gen_rmw_req_id = 
      RmwRequestId {
        writer_guid: self.client_guid,
        sequence_number: self.sequence_number(),
      };
    let req_wrapper = 
      RequestWrapper::<S::Request>::new(
        self.service_mapping, 
        gen_rmw_req_id, 
        RepresentationIdentifier::CDR_LE,
        request)?;
    let write_opts_builder = WriteOptionsBuilder::new()
      .source_timestamp(Timestamp::now()); // always add source timestamp

    let write_opts_builder = 
      if self.service_mapping == ServiceMapping::Enhanced {
        write_opts_builder
      } else {
        write_opts_builder.related_sample_identity(SampleIdentity::from(gen_rmw_req_id))
      };
    let sent_rmw_req_id =
      self.request_sender.async_write_with_options(req_wrapper, write_opts_builder.build())
        .await
        .map( RmwRequestId::from )?;

    match self.service_mapping {
      ServiceMapping::Enhanced => Ok(sent_rmw_req_id),
      ServiceMapping::Basic |
      ServiceMapping::Cyclone => Ok(gen_rmw_req_id),
    }  
  }

  pub async fn async_receive_response(&self) -> dds::Result<(RmwRequestId, S::Response)> {
    let dcc_stream = self.response_receiver.as_async_stream();
    pin_mut!(dcc_stream);
    let (dcc_rw, _tail) : (Option<dds::Result<no_key::DeserializedCacheChange<ResponseWrapper<S::Response>>>> , _ )= 
      dcc_stream.into_future().await;

    match dcc_rw {
      None => Err(dds::Error::Internal{ reason:
        "SimpleDataReader value stream unexpectedly ended!".to_string() }),
        // This should never occur, because topic do not "end". 
      Some(Err(e)) => Err(e),
      Some(Ok(dcc)) => {
        let mi = MessageInfo::from(&dcc);
        let res_wrapper = dcc.into_value();
        let (ri,res) = res_wrapper
          .unwrap(self.service_mapping, mi , self.client_guid)?;
        Ok((ri,res))
      }
    } // match
  }


  fn increment_sequence_number(&self) {
    self
      .sequence_number_gen
      .fetch_add(1, atomic::Ordering::Acquire);
  }

  fn sequence_number(&self) -> request_id::SequenceNumber {
    SequenceNumber::from(self.sequence_number_gen.load(atomic::Ordering::Acquire)).into()
  }
}

impl<S> Evented for Client<S>
where
  S: 'static + Service,
{
  fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
    self.response_receiver.register(poll, token, interest, opts)
  }

  fn reregister(
    &self,
    poll: &Poll,
    token: Token,
    interest: Ready,
    opts: PollOpt,
  ) -> io::Result<()> {
    self.response_receiver.reregister(poll, token, interest, opts)
  }

  fn deregister(&self, poll: &Poll) -> io::Result<()> {
    self.response_receiver.deregister(poll)
  }
}


