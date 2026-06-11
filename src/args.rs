/// Parsed CLI arguments.
#[derive(Debug, Default)]
pub struct Args {
    /// Root pid to start tree from (default: 1).
    pub root_pid: i32,
    /// Show only branches containing this pid.
    pub pid_filter: Option<i32>,
    /// Show only branches containing processes owned by this username.
    pub user_filter: Option<String>,
    /// Maximum depth to display.
    pub max_depth: Option<usize>,
    /// Disable line truncation.
    pub wide: bool,
    /// Use ASCII tree characters instead of UTF-8.
    pub ascii: bool,
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn parse_args(mut argv: impl Iterator<Item = String>) -> Result<Args, ParseError> {
    // Skip program name.
    argv.next();

    let mut args = Args { root_pid: 1, ..Default::default() };

    let argv: Vec<String> = argv.collect();
    let mut i = 0;

    while i < argv.len() {
        match argv[i].as_str() {
            "-w" | "--wide" => {
                args.wide = true;
            }
            "--ascii" => {
                args.ascii = true;
            }
            "-p" => {
                i += 1;
                let val = argv.get(i).ok_or_else(|| ParseError("-p requires a pid".into()))?;
                args.pid_filter = Some(
                    val.parse::<i32>()
                        .map_err(|_| ParseError(format!("invalid pid: {}", val)))?,
                );
            }
            "-u" => {
                i += 1;
                let val = argv.get(i).ok_or_else(|| ParseError("-u requires a username".into()))?;
                args.user_filter = Some(val.clone());
            }
            "-l" => {
                i += 1;
                let val = argv.get(i).ok_or_else(|| ParseError("-l requires a number".into()))?;
                args.max_depth = Some(
                    val.parse::<usize>()
                        .map_err(|_| ParseError(format!("invalid depth: {}", val)))?,
                );
            }
            "-h" | "--help" => {
                return Err(ParseError(usage()));
            }
            s if s.starts_with('-') => {
                return Err(ParseError(format!("unknown flag: {}\n{}", s, usage())));
            }
            s => {
                // Positional: root pid.
                args.root_pid = s
                    .parse::<i32>()
                    .map_err(|_| ParseError(format!("invalid pid: {}", s)))?;
            }
        }
        i += 1;
    }

    Ok(args)
}

pub fn usage() -> String {
    "Usage: pstree-rs [-w] [--ascii] [-p pid] [-u user] [-l depth] [pid]

Options:
  pid        root pid to start from (default: 1)
  -p pid     show only branches containing pid
  -u user    show only branches containing processes owned by user
  -l depth   limit tree depth
  -w         wide output, no truncation
  --ascii    use ASCII tree characters
  -h         show this help"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Args, ParseError> {
        parse_args(s.split_whitespace().map(|s| s.to_string()))
    }

    #[test]
    fn defaults() {
        let a = parse("pstree-rs").unwrap();
        assert_eq!(a.root_pid, 1);
        assert!(!a.wide);
        assert!(!a.ascii);
        assert!(a.pid_filter.is_none());
        assert!(a.user_filter.is_none());
        assert!(a.max_depth.is_none());
    }

    #[test]
    fn positional_pid() {
        let a = parse("pstree-rs 42").unwrap();
        assert_eq!(a.root_pid, 42);
    }

    #[test]
    fn flags() {
        let a = parse("pstree-rs -w --ascii -p 99 -u didi -l 3").unwrap();
        assert!(a.wide);
        assert!(a.ascii);
        assert_eq!(a.pid_filter, Some(99));
        assert_eq!(a.user_filter.as_deref(), Some("didi"));
        assert_eq!(a.max_depth, Some(3));
    }

    #[test]
    fn missing_arg_for_flag() {
        assert!(parse("pstree-rs -p").is_err());
        assert!(parse("pstree-rs -u").is_err());
        assert!(parse("pstree-rs -l").is_err());
    }

    #[test]
    fn invalid_pid() {
        assert!(parse("pstree-rs -p abc").is_err());
        assert!(parse("pstree-rs notapid").is_err());
    }

    #[test]
    fn unknown_flag() {
        assert!(parse("pstree-rs --foo").is_err());
    }
}
