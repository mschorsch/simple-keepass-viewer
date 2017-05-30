use iron::prelude::*;
use iron::IronResult;
use iron::request::Request as IronRequest;
use iron::response::Response as IronResponse;
use iron::{AfterMiddleware, Handler};
use iron::status;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use iron::error::{IronError};
use router::{Router};
use hbs::Template;
use urlencoded::UrlEncodedBody;

use std::path;
use std::collections::BTreeMap;

use store;
use errors::*;

//
// Static asset handler
//

const BOOTSTRAP_JS: &'static str = include_str!("../templates/js/bootstrap.min.js");
const IE10_VIEWPORT_BUG_JS: &'static str = include_str!("../templates/js/ie10-viewport-bug-workaround.js");
const JQUERY_JS: &'static str = include_str!("../templates/js/jquery.min.js");
const BOOSTRAP_SHOW_HIDE_PASSWORD_JS: &'static str = include_str!("../templates/js/bootstrap-show-password.min.js");

const GLYPHICONS_HALFLINGS_REGULAR_TTF_FONTS: &'static [u8] = include_bytes!("../templates/fonts/glyphicons-halflings-regular.ttf");
const GLYPHICONS_HALFLINGS_REGULAR_WOFF_FONTS: &'static [u8] = include_bytes!("../templates/fonts/glyphicons-halflings-regular.woff");
const GLYPHICONS_HALFLINGS_REGULAR_WOFF2_FONTS: &'static [u8] = include_bytes!("../templates/fonts/glyphicons-halflings-regular.woff2");

const BOOTSTRAP_CSS: &'static str = include_str!("../templates/css/bootstrap.min.css");
const IE10_VIEWPORT_BUG_CSS: &'static str = include_str!("../templates/css/ie10-viewport-bug-workaround.css");
const APP_CSS: &'static str = include_str!("../templates/css/app.css");

lazy_static! {
    static ref APPLICATION_JS_MIME: Mime = Mime(TopLevel::Application, SubLevel::Javascript, vec![(Attr::Charset, Value::Utf8)]);
    static ref TEXT_CSS_MIME: Mime =  Mime(TopLevel::Text, SubLevel::Css, vec![(Attr::Charset, Value::Utf8)]);
    static ref TTF_MIME: Mime =  "application/font-ttf".parse().unwrap();
    static ref WOFF_MIME: Mime =  "application/font-woff".parse().unwrap();
    static ref WOFF2_MIME: Mime =  "application/font-woff2".parse().unwrap();
}

#[derive(Debug, Clone)]
pub struct AssetHandler;

impl AssetHandler {
    pub fn new() -> Self {
        AssetHandler
    }
}

impl Handler for AssetHandler {
    fn handle(&self, req: &mut IronRequest) -> IronResult<IronResponse> {
        let params = req.extensions.get::<Router>().unwrap();
        let static_resources = params.find("assets");

        match static_resources {
            // JS
            Some("bootstrap.min.js") => Ok(IronResponse::with((APPLICATION_JS_MIME.clone(), status::Ok, BOOTSTRAP_JS))),
            Some("ie10-viewport-bug-workaround.js") => Ok(IronResponse::with((APPLICATION_JS_MIME.clone(), status::Ok, IE10_VIEWPORT_BUG_JS))),
            Some("jquery.min.js") => Ok(IronResponse::with((APPLICATION_JS_MIME.clone(), status::Ok, JQUERY_JS))),
            Some("bootstrap-show-password.min.js") => Ok(IronResponse::with((APPLICATION_JS_MIME.clone(), status::Ok, BOOSTRAP_SHOW_HIDE_PASSWORD_JS))),

            // FONTS
            Some("glyphicons-halflings-regular.ttf") => Ok(IronResponse::with((TTF_MIME.clone(), status::Ok, GLYPHICONS_HALFLINGS_REGULAR_TTF_FONTS))),
            Some("glyphicons-halflings-regular.woff") => Ok(IronResponse::with((WOFF_MIME.clone(), status::Ok, GLYPHICONS_HALFLINGS_REGULAR_WOFF_FONTS))),
            Some("glyphicons-halflings-regular.woff2") => Ok(IronResponse::with((WOFF2_MIME.clone(), status::Ok, GLYPHICONS_HALFLINGS_REGULAR_WOFF2_FONTS))),

            // CSS
            Some("bootstrap.min.css") => Ok(IronResponse::with((TEXT_CSS_MIME.clone(), status::Ok, BOOTSTRAP_CSS))),
            Some("ie10-viewport-bug-workaround.css") => Ok(IronResponse::with((TEXT_CSS_MIME.clone(), status::Ok, IE10_VIEWPORT_BUG_CSS))),
            Some("app.css") => Ok(IronResponse::with((TEXT_CSS_MIME.clone(), status::Ok, APP_CSS))),

            _ => Ok(IronResponse::with(status::NotFound)),
        }
    }
}

//
// Index
//

pub const MAIN_VIEW: &'static str = "main_view";
pub const FILES_VIEW: &'static str = "files_view";
pub const ENTRIES_VIEW: &'static str = "entries_view";
pub const ERROR_VIEW: &'static str = "error_view";

#[derive(Debug, Serialize)]
struct FileViewData<'a> {
    parent: &'a str,
    filenames: Vec<String>,
}

#[derive(Debug)]
pub struct FilesViewHandler {
    directory: String,
}

impl FilesViewHandler {

    pub fn new<I: Into<String>>(directory: I) -> Self {
        FilesViewHandler {directory: directory.into()}
    }
}

impl Handler for FilesViewHandler {

    fn handle(&self, _: &mut IronRequest) -> IronResult<IronResponse> {
        match store::find_all_keepass_files(&self.directory) {
            Ok(keepass_files) => {
                let filenames = keepass_files
                    .into_iter()
                    .map(|keepass_file| keepass_file.filename)
                    .collect();

                let file_view_data = FileViewData {
                    parent: MAIN_VIEW,
                    filenames: filenames,
                };

                let mut resp = IronResponse::new();
                resp.set_mut(Template::new(FILES_VIEW, file_view_data)).set_mut(status::Ok);
                Ok(resp)
            },
            Err(err) => Err(IronError::new(err, status::InternalServerError)),
        }
    }
}

//
// Login / Entries
//

#[derive(Debug)]
struct LoginFormData {
    filename: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct EntriesViewData<'a> {
    parent: &'a str,
    entries: Vec<store::KeePassEntry>,
}

#[derive(Debug)]
pub struct EntriesViewHandler {
    directory: String,
}

impl EntriesViewHandler {

    pub fn new<I: Into<String>>(directory: I) -> Self {
        EntriesViewHandler {directory: directory.into()}
    }
}

impl Handler for EntriesViewHandler {

    fn handle(&self, req: &mut IronRequest) -> IronResult<IronResponse> {
        match decode_login_form_data(req) {
            Ok(logindata) => {
                let filepath = path::Path::new(&self.directory).join(&logindata.filename);
                let keepass_file = store::KeePassFile::new(filepath, logindata.filename.clone());

                match keepass_file.get_entries(&logindata.password) {
                    Ok(entries) => {
                        let entries_view_data = EntriesViewData {
                            parent: MAIN_VIEW,
                            entries: entries,
                        };

                        let mut resp = IronResponse::new();
                        resp.set_mut(Template::new(ENTRIES_VIEW, entries_view_data)).set_mut(status::Ok);
                        Ok(resp)
                    },
                    Err(err) => Err(IronError::new(err, status::Unauthorized)),
                }                
            },
            Err(err) => Err(IronError::new(err, status::BadRequest)),
        }
    }
}

fn decode_login_form_data(req: &mut IronRequest) -> Result<LoginFormData> {
    let hashmap = &(req.get_ref::<UrlEncodedBody>().chain_err(|| "could not decode body")?);

    let filename: String = hashmap.get("filename")
        .and_then(|values| values.get(0).map(|s| s.to_owned()))
        .ok_or_else::<ErrorKind,_>(|| ErrorKind::InvalidLoginData(String::from("invalid filename")).into())?;

    let password: String = hashmap.get("password")
        .and_then(|values| values.get(0).map(|s| s.to_owned()))
        .ok_or_else::<ErrorKind,_>(|| ErrorKind::InvalidLoginData(String::from("invalid password")).into())?;

    Ok(LoginFormData{
        filename: filename,
        password: password
    })
}

//
// Error middlware
//

pub struct ErrorHandler;

impl AfterMiddleware for ErrorHandler {

    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(statuscode) => {
                let mut resp = IronResponse::new();
                let errormsg = format!("Error: \"{}\"", err.error.description());

                resp.set_mut(Template::new(ERROR_VIEW, get_parent_view(&errormsg)))
                    .set_mut(statuscode);
                Ok(resp)
            },
            None => Err(err),
        }
    }
}

fn get_parent_view<'a>(description: &'a str) -> BTreeMap<&'static str, &'a str> {
    let mut parent = BTreeMap::new();
    parent.insert("parent", MAIN_VIEW);
    parent.insert("errormsg", description);
    parent
}