macro_rules! to_string_opt_as_str {
    ($opt:expr) => {
        $opt.as_ref().map(::std::string::ToString::to_string).as_ref().map(|s| s.as_str()).unwrap_or("")
    };
}
