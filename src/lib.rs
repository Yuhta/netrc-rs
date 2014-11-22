use std::default::Default;
use std::iter::Iterator;
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

struct NetrcParser<'a> {
    input: Iterator<String>,
    lineno: uint,
}

impl<'a> NetrcParser<'a> {
    pub fn with_input<T: Reader>(input: &mut T) -> IoResult<NetrcParser> {
        let input = try!(input.read_to_string());
        Ok(NetrcParser {
            input: input.graphemes(true),
            lineno: Default::default(),
        })
    }

    pub fn with_file(file: &Path) -> IoResult<NetrcParser> {
        let mut f = try!(File::open(file));
        NetrcParser::with_input(&mut f)
    }

    // pub fn next_token(&mut self) -> &str {
    //     match self.input.
    // }

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
