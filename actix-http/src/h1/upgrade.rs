use std::task::Poll;

use actix_codec::Framed;
use actix_service::{Service, ServiceFactory};
use futures_util::future::{ready, Ready};

use crate::error::Error;
use crate::h1::Codec;
use crate::request::Request;

pub struct UpgradeHandler;

impl<T> ServiceFactory<(Request, Framed<T, Codec>)> for UpgradeHandler {
    type Response = ();
    type Error = Error;
    type Config = ();
    type Service = UpgradeHandler;
    type InitError = Error;
    type Future = Ready<Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        unimplemented!()
    }
}

impl<T> Service<(Request, Framed<T, Codec>)> for UpgradeHandler {
    type Response = ();
    type Error = Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    actix_service::always_ready!();

    fn call(&self, _: (Request, Framed<T, Codec>)) -> Self::Future {
        ready(Ok(()))
    }
}
