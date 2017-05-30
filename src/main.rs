#![recursion_limit = "1024"]
#![allow(unknown_lints)]

// external crates
extern crate iron;
extern crate hyper_native_tls;
extern crate router;
extern crate handlebars_iron as hbs;
extern crate urlencoded;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate error_chain;
extern crate keepass;
extern crate clap;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate log4rs;

// modules
mod errors;
mod store;
mod handler;

// uses
use iron::prelude::*;
use router::Router;
use hbs::{HandlebarsEngine, MemorySource};
use hyper_native_tls::NativeTlsServer;

use log::LogLevelFilter;
use log4rs::config::{Config, Logger, Root, Appender};
use log4rs::append::console::ConsoleAppender;
use log4rs::encode::pattern::PatternEncoder;

#[allow(unused_imports)] use errors::*;
use clap::{Arg, App, ArgMatches};

use std::collections::BTreeMap;
use std::str::FromStr;
use std::path::Path;
use std::result::Result as StdResult;
use std::net::SocketAddr;

// TODOS
// - documentation

#[allow(needless_pass_by_value)]
fn is_dir(dir: String) -> StdResult<(), String> {
    if Path::new(&dir).is_dir() {
        Ok(())        
    } else {
        Err(String::from("directory not found"))
    }
}

#[allow(needless_pass_by_value)]
fn is_file(file: String) -> StdResult<(), String> {
    if Path::new(&file).is_file() {
        Ok(())        
    } else {
        Err(String::from("file not found"))
    }
}

#[allow(needless_pass_by_value)]
fn is_socket_addr(address: String) -> StdResult<(), String> {
    address.parse::<SocketAddr>()
        .and_then(|_| Ok(()))
        .or_else(|_| Err(String::from("invalid socket address")))
}

fn match_cmd_arguments<'a>() -> ArgMatches<'a> {
    App::new("Simple KeePass Viewer")
        .version("0.1.0")
        .author("Matthias Schorsch")
        .arg(Arg::with_name("address")
            .short("a")
            .long("address")
            .value_name("ADDRESS")
            .help("Sets a custom address")
            .takes_value(true)
            .validator(is_socket_addr)
            .default_value("127.0.0.1:8000"))
        .arg(Arg::with_name("directory")
            .short("d")
            .long("directory")
            .value_name("DIRECTORY")
            .help("Sets a custom directory")
            .takes_value(true)
            .validator(is_dir)
            .default_value("./"))
        .arg(Arg::with_name("ssl_cert")
            .long("ssl_cert")
            .value_name("PKCS12")
            .help("Sets a pkcs12 certificate")
            .takes_value(true)
            .validator(is_file)
            .required_unless("insecure")
            .conflicts_with("insecure"))            
        .arg(Arg::with_name("ssl_cert_pw")
            .long("ssl_cert_pw")
            .value_name("PKCS12 PASSWORD")
            .help("Sets a password for the pkcs12 certificate")
            .takes_value(true)
            .required_unless("insecure")
            .conflicts_with("insecure"))
        .arg(Arg::with_name("insecure")
            .long("insecure")
            .value_name("INSECURE")
            .help("http instead of https")
            .takes_value(false)
            .conflicts_with_all(&["ssl_cert", "ssl_cert_pw"]))     
        .arg(Arg::with_name("loglevel")
            .long("loglevel")
            .value_name("LOGLEVEL")
            .takes_value(true)
            .possible_values(&["error", "warn", "info", "debug", "trace"])
            .default_value("info")
            .help("Sets a custom log level"))            
        .get_matches()
}

fn init_logger(log_level: Option<&str>) -> Result<LogLevelFilter> {
    let level = LogLevelFilter::from_str(log_level.unwrap_or("info")).unwrap_or(LogLevelFilter::Info);

    // Appender
    let stdout_appender = Appender::builder()
        .build(String::from("stdout"),
               Box::new(ConsoleAppender::builder()
                   .encoder(Box::new(PatternEncoder::new("{h({l})} {m}{n}")))
                   .build()));

    // Root logger
    let root = Root::builder().appender("stdout".to_owned()).build(level);

    // Logger
    let keepass_viewer_logger = Logger::builder().build("keepass_viewer".to_owned(), level);

    let config = Config::builder()
        .appender(stdout_appender)
        .logger(keepass_viewer_logger)
        .build(root)?;

    Ok(log4rs::init_config(config).and(Ok(level))?)
}

fn init_router(keepass_dir: &str) -> Result<Router> {
    // Router
    let mut router = Router::new();

    // ** GET handler
    router.get("/", handler::FilesViewHandler::new(keepass_dir), "files");

    // Asset handler
    let asset_handler = handler::AssetHandler::new();
    router.get(r"/js/:assets", asset_handler.clone(), "js_assets");
    router.get(r"/css/:assets", asset_handler.clone(), "css_assets");
    router.get(r"/fonts/:assets", asset_handler.clone(), "fonts_assets");

    // ** POST handler
    router.post("/entries", handler::EntriesViewHandler::new(keepass_dir), "entries");

    Ok(router)
}

fn init_chain(router: Router) -> Chain {
    let mut chain = Chain::new(router);
    chain.link_after(handler::ErrorHandler);
    chain.link_after(init_handlebars());
    chain
}

fn init_handlebars() -> HandlebarsEngine {
    let mut hbse = HandlebarsEngine::new();

    let mut mem_templates = BTreeMap::new();
    mem_templates.insert(handler::MAIN_VIEW.to_owned(), include_str!("../templates/main.html.hbs").to_owned());
    mem_templates.insert(handler::FILES_VIEW.to_owned(), include_str!("../templates/files.html.hbs").to_owned());
    mem_templates.insert(handler::ENTRIES_VIEW.to_owned(), include_str!("../templates/entries.html.hbs").to_owned());
    mem_templates.insert(handler::ERROR_VIEW.to_owned(), include_str!("../templates/error.html.hbs").to_owned());

    // add a memory based source
    hbse.add(Box::new(MemorySource(mem_templates)));    

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }
    hbse
}

fn main() {
    println!(" _____ _                 _        _   __         ______               _   _ _                        ");
    println!("/  ___(_)               | |      | | / /         | ___ \\             | | | (_)                       ");
    println!("\\ `--. _ _ __ ___  _ __ | | ___  | |/ /  ___  ___| |_/ /_ _ ___ ___  | | | |_  _____      _____ _ __ ");
    println!(" `--. \\ | '_ ` _ \\| '_ \\| |/ _ \\ |    \\ / _ \\/ _ \\  __/ _` / __/ __| | | | | |/ _ \\ \\ /\\ / / _ \\ '__|");
    println!("/\\__/ / | | | | | | |_) | |  __/ | |\\  \\  __/  __/ | | (_| \\__ \\__ \\ \\ \\_/ / |  __/\\ V  V /  __/ |   ");
    println!("\\____/|_|_| |_| |_| .__/|_|\\___| \\_| \\_/\\___|\\___\\_|  \\__,_|___/___/  \\___/|_|\\___| \\_/\\_/ \\___|_|   ");
    println!("                  | |                                                                                ");
    println!("                  |_|                                                                                ");

    // CLI
    let arg_matches = match_cmd_arguments();

    // Logger
    init_logger(arg_matches.value_of("loglevel"))
        .map(|level| info!("✔ Logger with loglevel '{}' initialized.", level))
        .expect("could not initialize log");

    // Router / Chain
    let keepass_dir = arg_matches.value_of("directory").expect("invalid directory");
    info!("✔ Directory '{}' found.", keepass_dir);

    let router = init_router(keepass_dir).expect("could not initialize router");
    let chain = init_chain(router);

    let address = arg_matches.value_of("address").expect("invalid server address");

    if arg_matches.is_present("insecure") {
        // HTTP
        warn!("Insecure mode active.");
        info!("➥ Running 'http://{}' ...", address);
        Iron::new(chain).http(address).unwrap();
    } else {
        // HTTPS
        let ssl_cert = arg_matches.value_of("ssl_cert").expect("pkcs12 certificate not found");
        let ssl_pw = arg_matches.value_of("ssl_cert_pw").expect("certificate password not found");
        let ssl = NativeTlsServer::new(ssl_cert, ssl_pw).unwrap();
        info!("✔ SSL with certificate '{}' initialized.", ssl_cert);

        info!("➥ Running 'https://{}' ...", address);
        Iron::new(chain).https(address, ssl).unwrap();
    }
}