
use crate::message::Message;
use std::marker::PhantomData;

pub trait Service {
    type Request: Message;
    type Response: Message;
}


pub struct Server<S:Service> {
    service_type: PhantomData<S>,
}

// impl Evented for Server {

// }

pub struct Client<S:Service> {
    service_type: PhantomData<S>,

}