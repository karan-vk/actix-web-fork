use std::future::Future;
use std::time::Instant;

use actix_http::Response;
use actix_web::http::StatusCode;
use actix_web::test::TestRequest;
use actix_web::{error, Error, HttpRequest, HttpResponse, Responder};
use criterion::{criterion_group, criterion_main, Criterion};
use futures_util::future::{ready, Either, Ready};

// responder simulate the old responder trait.
trait FutureResponder {
    type Error;
    type Future: Future<Output = Result<HttpResponse, Self::Error>>;

    fn future_respond_to(self, req: &HttpRequest) -> Self::Future;
}

// a simple option responder type.
struct OptionResponder<T>(Option<T>);

// a simple wrapper type around string
struct StringResponder(String);

impl FutureResponder for StringResponder {
    type Error = Error;
    type Future = Ready<Result<Response, Self::Error>>;

    fn future_respond_to(self, _: &HttpRequest) -> Self::Future {
        // this is default builder for string response in both new and old responder trait.
        ready(Ok(Response::build(StatusCode::OK)
            .content_type("text/plain; charset=utf-8")
            .body(self.0)))
    }
}

impl<T> FutureResponder for OptionResponder<T>
where
    T: FutureResponder,
    T::Future: Future<Output = Result<Response, Error>>,
{
    type Error = Error;
    type Future = Either<T::Future, Ready<Result<HttpResponse, Self::Error>>>;

    fn future_respond_to(self, req: &HttpRequest) -> Self::Future {
        match self.0 {
            Some(t) => Either::Left(t.future_respond_to(req)),
            None => Either::Right(ready(Err(error::ErrorInternalServerError("err")))),
        }
    }
}

impl Responder for StringResponder {
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        Response::build(StatusCode::OK)
            .content_type("text/plain; charset=utf-8")
            .body(self.0)
    }
}

impl<T: Responder> Responder for OptionResponder<T> {
    fn respond_to(self, req: &HttpRequest) -> HttpResponse {
        match self.0 {
            Some(t) => t.respond_to(req),
            None => Response::from_error(error::ErrorInternalServerError("err")),
        }
    }
}

fn future_responder(c: &mut Criterion) {
    let rt = actix_rt::System::new();
    let req = TestRequest::default().to_http_request();

    c.bench_function("future_responder", move |b| {
        b.iter_custom(|_| {
            let futs = (0..100_000).map(|_| async {
                StringResponder(String::from("Hello World!!"))
                    .future_respond_to(&req)
                    .await
            });

            let futs = futures_util::future::join_all(futs);

            let start = Instant::now();

            let _res = rt.block_on(async { futs.await });

            start.elapsed()
        })
    });
}

fn responder(c: &mut Criterion) {
    let rt = actix_rt::System::new();
    let req = TestRequest::default().to_http_request();
    c.bench_function("responder", move |b| {
        b.iter_custom(|_| {
            let responders =
                (0..100_000).map(|_| StringResponder(String::from("Hello World!!")));

            let start = Instant::now();
            let _res = rt.block_on(async {
                // don't need runtime block on but to be fair.
                responders.map(|r| r.respond_to(&req)).collect::<Vec<_>>()
            });

            start.elapsed()
        })
    });
}

criterion_group!(responder_bench, future_responder, responder);
criterion_main!(responder_bench);
