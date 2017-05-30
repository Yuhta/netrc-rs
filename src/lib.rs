#[macro_use] extern crate error_chain;

pub use errors::*;

use std::str::FromStr;
use std::io::{BufRead, BufReader, Read};

mod errors;

#[derive(Debug, PartialEq, Clone)]
pub struct Macro(pub String, pub String);

#[derive(Debug, PartialEq, Clone)]
pub struct Host(pub String, pub Machine);

/// Represents a machine record of a Netrc file
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Machine {
    pub login: String,
    pub password: Option<String>,
    pub account: Option<String>,
    pub port: Option<u16>,
}

/// Represents a Netrc entry
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Netrc {
    pub hosts: Vec<Host>,
    pub default: Option<Machine>,
    pub macros: Vec<Macro>,
}

impl Netrc {
    /// Parse a `Netrc` object from byte stream.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use netrc::Netrc;
    /// use std::io::Cursor;
    ///
    /// let input: Cursor<&[u8]> =
    ///   Cursor::new(b"machine example.com login foo password bar");
    /// Netrc::parse(input).expect("Parse Failed");
    /// ```
    pub fn parse<A: Read>(buf: A) -> Result<Netrc> {
        let mut netrc: Netrc = Default::default();
        let mut lexer = Lexer::new(BufReader::new(buf));
        let mut current_machine = MachineRef::Nothing;
        loop {
            match lexer.next_word() {
                None         => break,
                Some(Err(e)) => return Err(e),
                Some(Ok(w))  => current_machine = netrc.parse_entry(&mut lexer, &w, current_machine)?,
            }
        }
        Ok(netrc)
    }

    fn parse_entry<A: BufRead>(&mut self,
                               lexer: &mut Lexer<A>,
                               item: &str,
                               current_machine: MachineRef) -> Result<MachineRef> {
        macro_rules! with_current_machine {
            ($entry: expr, $machine: ident, $body: block) => {
                match self.find_machine(&current_machine) {
                    Some($machine) => {
                        $body;
                        Ok(current_machine)
                    }
                    None =>
                        Err(ErrorKind::Parse(format!("No machine defined for {}",
                                                 $entry),
                                         lexer.lnum).into()),
                }
            }
        }

        match item {
            "machine" => {
                let host_name = lexer.next_word_or_err()?;
                self.hosts.push(Host(host_name, Default::default()));
                Ok(MachineRef::Host(self.hosts.len() - 1))
            }
            "default" => {
                self.default = Some(Default::default());
                Ok(MachineRef::Default)
            }
            "login" => with_current_machine!("login", m, {
                m.login = lexer.next_word_or_err()?;
            }),
            "password" => with_current_machine!("password", m, {
                m.password = Some(lexer.next_word_or_err()?);
            }),
            "account" => with_current_machine!("account", m, {
                m.account = Some(lexer.next_word_or_err()?);
            }),
            "port" => with_current_machine!("port", m, {
                let port = lexer.next_word_or_err()?;
                match port.parse() {
                    Ok(port) => m.port = Some(port),
                    Err(_)   => {
                        let msg = format!("Unable to parse port number `{}'",
                                          port);
                        return Err(ErrorKind::Parse(msg, lexer.lnum).into());
                    }
                }
            }),
            "macdef" => {
                let name = lexer.next_word_or_err()?;
                let cmds = lexer.next_subcommands()?;
                self.macros.push(Macro(name, cmds));
                Ok(MachineRef::Nothing)
            }
            _ => Err(ErrorKind::Parse(format!("Unknown entry `{}'", item),
                                  lexer.lnum).into()),
        }
    }

    fn find_machine(&mut self,
                    reference: &MachineRef) -> Option<&mut Machine> {
        match *reference {
            MachineRef::Nothing => None,
            MachineRef::Default => self.default.as_mut(),
            MachineRef::Host(n) => Some(&mut self.hosts[n].1),
        }
    }
}

impl FromStr for Netrc {
    type Err = Error;

    fn from_str(s: &str) -> Result<Netrc> {
        Netrc::parse(s.as_bytes())
    }
}

enum MachineRef {
    Nothing,
    Default,
    Host(usize),
}

struct Tokens {
    buf: String,
    cur: usize,
}

impl Tokens {
    fn new(buf: String) -> Tokens {
        Tokens { buf: buf, cur: 0 }
    }

    fn empty() -> Tokens {
        Tokens::new("".to_string())
    }

    fn remaining(&self) -> &str {
        &self.buf[self.cur..]
    }

    fn next(&mut self) -> Option<String> {
        let mut cur = self.cur;
        for _ in self.remaining().chars().take_while(|c| c.is_whitespace()) {
            cur += 1;
        }
        self.cur = cur;
        if cur < self.buf.len() {
            let mut s = String::new();
            for c in self.remaining().chars().take_while(|c| !c.is_whitespace()) {
                cur += 1;
                s.push(c);
            }
            self.cur = cur;
            Some(s)
        } else {
            None
        }
    }
}

struct Lexer<A> {
    buf: A,
    line: Tokens,
    lnum: usize,
}

impl<A: BufRead> Lexer<A> {
    fn new(buf: A) -> Lexer<A> {
        Lexer { buf: buf, line: Tokens::empty(), lnum: 0 }
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        let r = self.buf.read_line(buf)?;
        if r > 0 {
            self.lnum += 1;
        }
        Ok(r)
    }

    fn refill(&mut self) -> Result<usize> {
        let mut line = String::new();
        let n = self.read_line(&mut line)?;
        self.line = Tokens::new(line);
        Ok(n)
    }

    fn next_word(&mut self) -> Option<Result<String>> {
        loop {
            match self.line.next() {
                Some(w) => return Some(Ok(w)),
                None    => match self.refill() {
                    Ok(0)  => return None,
                    Ok(_)  => (),
                    Err(e) => return Some(Err(e)),
                },
            }
        }
    }

    fn next_word_or_err(&mut self) -> Result<String> {
        match self.next_word() {
            Some(w) => w,
            None    => Err(ErrorKind::Parse("Unexpected end of file".to_string(),
                                        self.lnum).into()),
        }
    }

    fn next_subcommands(&mut self) -> Result<String> {
        let mut cmds = self.line.remaining().to_string();
        self.line = Tokens::empty();
        loop {
            match self.read_line(&mut cmds) {
                Ok(0...1) => return Ok(cmds),
                Ok(_)     => (),
                Err(e)    => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_simple() {
        let input = "machine example.com
                             login test
                             password p@ssw0rd
                             port 42";
        let netrc = Netrc::parse(input.as_bytes()).unwrap();
        let expected = Netrc {
            hosts: vec![
                    Host("example.com".into(), Machine {
                        login: "test".into(),
                        password: Some("p@ssw0rd".into()),
                        port: Some(42),
                        ..Default::default()
                    })
            ],
            ..Default::default()
        };
        assert_eq!(netrc, expected);
    }

    #[test]
    fn parse_macdef() {
        let input = "machine host1.com login login1
                     macdef uploadtest
                            cd /pub/tests
                            bin
                            put filename.tar.gz
                            quit

                     machine host2.com login login2";
        let netrc = Netrc::from_str(input).unwrap();

        let expected_macro = Macro("uploadtest".into(), "
                            cd /pub/tests
                            bin
                            put filename.tar.gz
                            quit\n\n".into());
        let expected_hosts = vec![
            Host("host1.com".into(), Machine { login: "login1".into(), ..Default::default() }),
            Host("host2.com".into(), Machine { login: "login2".into(), ..Default::default() }),
        ];

        let expected = Netrc {
            hosts: expected_hosts,
            macros: vec![expected_macro],
            ..Default::default()
        };

        assert_eq!(netrc, expected);

    }

    #[test]
    fn parse_default() {
        let input = "machine example.com login test
                     default login def";
        let netrc = Netrc::parse(input.as_bytes()).unwrap();

        let expected = Netrc {
            hosts: vec![
                Host(
                        "example.com".into(),
                        Machine {
                            login: "test".into(),
                            ..Default::default()
                        }
                )
            ],
            default: Some(Machine {
                login: "def".into(),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(netrc, expected);
    }

    #[test]
    fn parse_error_unknown_entry() {
        let input = "machine foobar.com
                             foo";
        match Netrc::parse(input.as_bytes()).unwrap_err() {
            Error(ErrorKind::Parse(msg, lnum), _) => {
                assert_eq!(msg, "Unknown entry `foo'");
                assert_eq!(lnum, 2);
            }
            e => panic!("Wrong Error type: {:?}", e),
        }
    }

    #[test]
    fn parse_error_unexpected_eof() {
        let input = "machine foobar.com
                             password quux
                             login";
        match Netrc::parse(input.as_bytes()).unwrap_err() {
            Error(ErrorKind::Parse(msg, lnum), _) => {
                assert_eq!(msg, "Unexpected end of file");
                assert_eq!(lnum, 3);
            }
            e => panic!("Wrong Error type: {:?}", e),
        }
    }

    #[test]
    fn parse_error_no_machine() {
        let input = "password quux login foo";
        match Netrc::parse(input.as_bytes()).unwrap_err() {
            Error(ErrorKind::Parse(msg, lnum), _) => {
                assert_eq!(msg, "No machine defined for password");
                assert_eq!(lnum, 1);
            }
            e => panic!("Wrong Error type: {:?}", e),
        }
    }

    #[test]
    fn parse_error_port() {
        let input = "machine foo.com login bar port quux";
        match Netrc::parse(input.as_bytes()).unwrap_err() {
            Error(ErrorKind::Parse(msg, lnum), _) => {
                assert_eq!(msg, "Unable to parse port number `quux'");
                assert_eq!(lnum, 1);
            }
            e => panic!("Wrong Error type: {:?}", e),
        }
    }
}
