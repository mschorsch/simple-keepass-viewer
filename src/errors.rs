error_chain!{
    
    foreign_links {
        KeepassDBErr(::keepass::OpenDBError);
        IoErr(::std::io::Error);
        SetLogErr(::log::SetLoggerError);
        Log4RsConfigErr(::log4rs::config::Errors);
    }

    errors {
        InvalidLoginData(s: String) {
            description("invalid login data")
            display("invalid login data: '{}'", s)
        }
    }
}