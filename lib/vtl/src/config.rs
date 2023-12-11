pub struct Config {
    // TODO: output for log
    pub output: usize,

    // add extra functions.
    //
    // builtin functions is added by default.
    pub functions: Vec<dyn Function>,
}

