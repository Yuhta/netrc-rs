use std::default::Default;
use std::io::{IoResult};
use std::io::fs::File;

// use std::os;
// use std::path::Path;

type Macro = (String, String);
type Host = (String, Machine);

struct Machine {
    login: String,
    password: String,
    account: String,
}

#[deriving(Default)]
struct Netrc {
    pub hosts: Vec<Host>,
    macros: Vec<Macro>,
}

struct NetrcParser<T> {
    input: Option<T>,
    lineno: uint,
}

impl<T: Reader> NetrcParser<T> {
    pub fn new() -> NetrcParser<T> {
        NetrcParser {
            input: None,
            lineno: Default::default(),
        }
    }

    pub fn with_input(input: T) -> NetrcParser<T> {
        NetrcParser {
            input: Some(input),
            lineno: Default::default(),
        }
    }

    pub fn with_file(file: &Path) -> IoResult<NetrcParser<File>> {
        let f = try!(File::open(file));
        Ok(NetrcParser::with_input(f))
    }

    pub fn parse(&self) -> Netrc {
        Default::default()
    }
}

#[cfg(test)]
mod test {
    use super::{NetrcParser, Netrc};
    use std::io::BufReader;

    #[test]
    fn test_simple() {
        let input = "machine example.com
    login test
    password p@ssw0rd
";
        let netrc = NetrcParser::with_input(BufReader::new(input.as_bytes())).parse();
    }
}
