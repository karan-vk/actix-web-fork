use std::{fmt, io, path::PathBuf, rc::Rc, task::Poll};

use actix_service::Service;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    error::Error,
    guard::Guard,
    http::{header, Method},
    HttpResponse,
};
use futures_util::future::{ok, Either, LocalBoxFuture, Ready};

use crate::{
    named, Directory, DirectoryRenderer, FilesError, HttpService, MimeOverride, NamedFile,
    PathBufWrap,
};

/// Assembled file serving service.
pub struct FilesService {
    pub(crate) directory: PathBuf,
    pub(crate) index: Option<String>,
    pub(crate) show_index: bool,
    pub(crate) redirect_to_slash: bool,
    pub(crate) default: Option<HttpService>,
    pub(crate) renderer: Rc<DirectoryRenderer>,
    pub(crate) mime_override: Option<Rc<MimeOverride>>,
    pub(crate) file_flags: named::Flags,
    pub(crate) guards: Option<Rc<dyn Guard>>,
    pub(crate) hidden_files: bool,
}

type FilesServiceFuture = Either<
    Ready<Result<ServiceResponse, Error>>,
    LocalBoxFuture<'static, Result<ServiceResponse, Error>>,
>;

impl FilesService {
    fn handle_err(&self, e: io::Error, req: ServiceRequest) -> FilesServiceFuture {
        log::debug!("Failed to handle {}: {}", req.path(), e);

        if let Some(ref default) = self.default {
            Either::Right(default.call(req))
        } else {
            Either::Left(ok(req.error_response(e)))
        }
    }
}

impl fmt::Debug for FilesService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FilesService")
    }
}

impl Service<ServiceRequest> for FilesService {
    type Response = ServiceResponse;
    type Error = Error;
    type Future = FilesServiceFuture;

    actix_service::always_ready!();

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let is_method_valid = if let Some(guard) = &self.guards {
            // execute user defined guards
            (**guard).check(req.head())
        } else {
            // default behavior
            matches!(*req.method(), Method::HEAD | Method::GET)
        };

        if !is_method_valid {
            return Either::Left(ok(req.into_response(
                actix_web::HttpResponse::MethodNotAllowed()
                    .insert_header(header::ContentType(mime::TEXT_PLAIN_UTF_8))
                    .body("Request did not meet this resource's requirements."),
            )));
        }

        let real_path =
            match PathBufWrap::parse_path(req.match_info().path(), self.hidden_files) {
                Ok(item) => item,
                Err(e) => return Either::Left(ok(req.error_response(e))),
            };

        // full file path
        let path = match self.directory.join(&real_path).canonicalize() {
            Ok(path) => path,
            Err(e) => return self.handle_err(e, req),
        };

        if path.is_dir() {
            if let Some(ref redir_index) = self.index {
                if self.redirect_to_slash && !req.path().ends_with('/') {
                    let redirect_to = format!("{}/", req.path());

                    return Either::Left(ok(req.into_response(
                        HttpResponse::Found()
                            .insert_header((header::LOCATION, redirect_to))
                            .body("")
                            .into_body(),
                    )));
                }

                let path = path.join(redir_index);

                match NamedFile::open(path) {
                    Ok(mut named_file) => {
                        if let Some(ref mime_override) = self.mime_override {
                            let new_disposition =
                                mime_override(&named_file.content_type.type_());
                            named_file.content_disposition.disposition = new_disposition;
                        }
                        named_file.flags = self.file_flags;

                        let (req, _) = req.into_parts();
                        let res = named_file.into_response(&req);
                        Either::Left(ok(ServiceResponse::new(req, res)))
                    }
                    Err(e) => self.handle_err(e, req),
                }
            } else if self.show_index {
                let dir = Directory::new(self.directory.clone(), path);

                let (req, _) = req.into_parts();
                let x = (self.renderer)(&dir, &req);

                match x {
                    Ok(resp) => Either::Left(ok(resp)),
                    Err(e) => Either::Left(ok(ServiceResponse::from_err(e, req))),
                }
            } else {
                Either::Left(ok(ServiceResponse::from_err(
                    FilesError::IsDirectory,
                    req.into_parts().0,
                )))
            }
        } else {
            match NamedFile::open(path) {
                Ok(mut named_file) => {
                    if let Some(ref mime_override) = self.mime_override {
                        let new_disposition = mime_override(&named_file.content_type.type_());
                        named_file.content_disposition.disposition = new_disposition;
                    }
                    named_file.flags = self.file_flags;

                    let (req, _) = req.into_parts();
                    let res = named_file.into_response(&req);
                    Either::Left(ok(ServiceResponse::new(req, res)))
                }
                Err(e) => self.handle_err(e, req),
            }
        }
    }
}
