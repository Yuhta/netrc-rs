error_chain!{
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        Parse(t: String, u: usize)
    }
}


