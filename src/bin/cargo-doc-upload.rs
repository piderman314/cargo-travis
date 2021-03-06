extern crate cargo;
extern crate cargo_travis;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

use std::env;
use cargo::util::{CargoError, Config, CliResult};

pub const USAGE: &'static str = ("
Upload built rustdoc documentation to GitHub pages.

Usage:
    cargo doc-upload [options] [--] [<args>...]

Options:
    -V, --version                Print version info and exit
    --branch NAME ...            Only publish documentation for these branches
                                 Defaults to only the `master` branch
    --token TOKEN                Use the specified GitHub token to publish documentation
                                 If unspecified, checks $GH_TOKEN then attempts to use SSH endpoint
    --message MESSAGE            The message to include in the commit
    --deploy BRANCH              Deploy to the given branch [default: gh-pages]
");

#[derive(Deserialize)]
pub struct Options {
    flag_version: bool,
    flag_branch: Vec<String>,
    flag_token: Option<String>,
    flag_message: Option<String>,
    flag_deploy: Option<String>,
}

fn execute(options: Options, _: &Config) -> CliResult {
    debug!("executing; cmd=cargo-doc-upload; env={:?}",
           env::args().collect::<Vec<_>>());

    if options.flag_version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let branches = if options.flag_branch.is_empty() {
        vec!["master".to_string()]
    } else {
        options.flag_branch
    };

    let branch = env::var("TRAVIS_BRANCH").expect("$TRAVIS_BRANCH not set");
    if !branches.contains(&branch) {
        println!("Skipping branch {}", branch);
        return Ok(());
    }

    let pull_request = env::var("TRAVIS_PULL_REQUEST").expect("$TRAVIS_PULL_REQUEST not set");
    if pull_request != "false" {
        println!("Skipping PR");
        return Ok(());
    }

    // TODO FEAT: Allow passing origin string
    let token = options.flag_token.or(env::var("GH_TOKEN").ok());
    let slug = env::var("TRAVIS_REPO_SLUG").expect("$TRAVIS_REPO_SLUG not set");
    let origin = if let Some(token) = token {
        format!("https://{}@github.com/{}.git", token, slug)
    } else {
        eprintln!("GitHub Personal Access Token was not provided in $GH_TOKEN or --token");
        eprintln!("Falling back to using the SSH endpoint");
        format!("git@github.com:{}.git", slug)
    };

    let message = options.flag_message.unwrap_or("Automatic Travis documentation build".to_string());
    let gh_pages = options.flag_deploy.unwrap_or("gh-pages".to_string());

    cargo_travis::doc_upload(&branch, &message, &origin, &gh_pages);
    Ok(())
}

fn main() {
    env_logger::init().unwrap();
    let config = match Config::default() {
        Ok(cfg) => cfg,
        Err(e) => {
            let mut shell = cargo::core::Shell::new();
            cargo::exit_with_error(e.into(), &mut shell)
        }
    };
    let result = (|| {
        let args: Vec<String> = try!(env::args_os()
            .map(|s| {
                s.into_string().map_err(|s| {
                    CargoError::from(format!("invalid unicode in argument: {:?}", s))
                })
            })
            .collect());
        let rest = &args;
        cargo::call_main_without_stdin(execute, &config, USAGE, rest, false)
    })();
    match result {
        Err(e) => cargo::exit_with_error(e, &mut *config.shell()),
        Ok(()) => {}
    }
}
