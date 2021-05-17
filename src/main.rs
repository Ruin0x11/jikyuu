extern crate chrono;
extern crate regex;
extern crate anyhow;
extern crate clap;
extern crate prettytable;
extern crate serde_json;
extern crate serde;

use std::cmp::Ordering;
use std::str::FromStr;
use std::string::ToString;
use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use chrono::{Local, Utc, Weekday, NaiveDate, NaiveTime, NaiveDateTime, Datelike, Duration, TimeZone};
use regex::Regex;
use git2::{Repository, Commit, BranchType};
use clap::{Arg, App, ArgMatches, crate_version, crate_authors, arg_enum, value_t};
use prettytable::{format, Table, row, cell};
use serde::{Deserialize, Serialize};

mod error;

enum CommitTimeBound {
    Always,
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    Date(NaiveDate),
}

impl CommitTimeBound {
    fn to_date_time(&self) -> Option<NaiveDateTime> {
        let zero = || NaiveTime::from_hms(0, 0, 0);

        match self {
            Self::Always => None,
            Self::Today => {
                let local = Local::today();
                let date = NaiveDate::from_ymd(local.year(), local.month(), local.day());
                Some(NaiveDateTime::new(date, zero()))
            }
            Self::Yesterday => {
                let local = Local::today();
                let date = NaiveDate::from_ymd(local.year(), local.month(), local.day()) - Duration::days(1);
                Some(NaiveDateTime::new(date, zero()))
            }
            Self::ThisWeek => {
                let local = Local::today();
                let date = NaiveDate::from_isoywd(local.year(), local.iso_week().week(), Weekday::Sun);
                Some(NaiveDateTime::new(date, zero()))
            }
            Self::LastWeek => {
                let local = Local::today();
                let date = NaiveDate::from_isoywd(local.year(), local.iso_week().week(), Weekday::Sun) - Duration::weeks(1);
                Some(NaiveDateTime::new(date, zero()))
            }
            Self::Date(date) => Some(NaiveDateTime::new(date.clone(), zero()))
        }
    }
}

impl FromStr for CommitTimeBound {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "today" => Ok(Self::Today),
            "yesterday" => Ok(Self::Yesterday),
            "thisweek" => Ok(Self::ThisWeek),
            "lastweek" => Ok(Self::LastWeek),
            x => {
                match NaiveDate::from_str(x) {
                    Ok(date) => Ok(Self::Date(date)),
                    Err(_) => Err(error::Error::new(format!("Could not parse date '{}' using YYYY-mm-dd format", x)))
                }
            }
        }
    }
}

impl ToString for CommitTimeBound {
    fn to_string(&self) -> String {
        match self {
            Self::Always => "always".into(),
            Self::Today => "today".into(),
            Self::Yesterday => "yesterday".into(),
            Self::ThisWeek => "thisweek".into(),
            Self::LastWeek => "lastweek".into(),
            Self::Date(date) => date.to_string()
        }
    }
}

arg_enum! {
    #[derive(PartialEq, Debug)]
    enum OutputFormat {
        Stdout,
        Json
    }
}

struct Config {
    /// Maximum time diff between 2 subsequent commits in minutes which are
    /// counted to be in the same coding "session"
    max_commit_diff: Duration,

    /// How many minutes should be added for the first commit of coding session
    first_commit_addition: Duration,

    /// Include commits since time x
    since: CommitTimeBound,

    /// Include commits until time x
    until: CommitTimeBound,

    // Include merge requests
    merge_requests: bool,

    /// Git repo
    git_repo_path: PathBuf,

    /// Aliases of emails for grouping the same activity as one person
    /// ("linus@torvalds.com": "linus@linux.com")
    email_aliases: HashMap<String, String>,

    /// Branch to filter commits by.
    branch: Option<String>,

    /// Type of branch that `branch` refers to.
    branch_type: BranchType,

    /// Output format.
    output_format: OutputFormat,

    // Display breakdown
    display_breakdown: bool
}

fn get_app<'a, 'b>() -> App<'a, 'b> {
    App::new("jikyuu")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Estimate the amount of time spent on a Git repository")
        .arg(Arg::with_name("max-commit-diff")
             .long("max-commit-diff")
             .short("d")
             .help("Maximum difference in minutes between commits counted to one session")
             .takes_value(true)
             .value_name("MINUTES")
             .default_value("120"))
        .arg(Arg::with_name("first-commit-add")
             .long("first-commit-add")
             .short("a")
             .help("How many minutes first commit of session should add to total")
             .takes_value(true)
             .value_name("MINUTES")
             .default_value("30"))
        .arg(Arg::with_name("since")
             .long("since")
             .short("s")
             .help("Analyze data since certain date")
             .takes_value(true)
             .value_name("always|today|yesterday|thisweek|lastweek|YYYY-mm-dd")
             .default_value("always"))
        .arg(Arg::with_name("until")
             .long("until")
             .short("u")
             .help("Analyze data until certain date")
             .takes_value(true)
             .value_name("always|today|yesterday|thisweek|lastweek|YYYY-mm-dd")
             .default_value("always"))
        .arg(Arg::with_name("email")
             .long("email")
             .short("e")
             .help("Associate all commits that have a secondary email with a primary email")
             .takes_value(true)
             .multiple(true)
             .number_of_values(1)
             .value_name("OTHER_EMAIL=MAIN_EMAIL"))
        .arg(Arg::with_name("merge-requests")
             .long("merge-requests")
             .short("m")
             .help("Include merge requests into calculation"))
        .arg(Arg::with_name("branch")
             .long("branch")
             .short("b")
             .takes_value(true)
             .help("Analyze only data on the specified branch"))
        .arg(Arg::with_name("branch-type")
             .long("branch-type")
             .short("t")
             .takes_value(true)
             .value_name("local|remote")
             .requires("branch")
             .help("Type of branch that `branch` refers to. `local` means refs/heads/, `remote` means refs/remotes/."))
        .arg(Arg::with_name("format")
             .long("format")
             .short("f")
             .takes_value(true)
             .possible_values(&OutputFormat::variants())
             .case_insensitive(true)
             .default_value("stdout"))
        .arg(Arg::with_name("REPO_PATH")
             .help("Root path of the Git repository to analyze.")
             .required(true)
             .default_value(".")
             .index(1))
        .arg(Arg::with_name("breakdown")
            .long("breakdown")
            .short("w")
            .help("Display number of work hours per day.")
            .takes_value(false))
}

fn parse_email_alias(s: &str) -> Result<(String, String)> {
    let mut splitter = s.splitn(2, "=");
    match splitter.next() {
        Some(a) => match splitter.next() {
            Some(b) => Ok((a.to_string(), b.to_string())),
            None => Err(anyhow!("Could not parse email alias '{}'", s))
        },
        None => Err(anyhow!("Could not parse email alias '{}'", s))
    }
}

fn get_commits<'repo>(branch: &Option<String>, branch_kind: BranchType, repo: &'repo Repository) -> Result<Vec<Commit<'repo>>> {
    let refs = repo.references()?;

    let ref_prefix = match branch_kind {
        BranchType::Local => "heads",
        BranchType::Remote => "remotes",
    };

    let branch_refs = match branch {
        Some(b) => {
            let s = format!("refs/{}/{}", ref_prefix, b);
            let mut vec = Vec::new();
            for r in refs {
                let r = r?;
                let name = r.name();
                if let Some(name) = name {
                    if name == s {
                        vec.push(r)
                    }
                }
            }
            vec
        }
        None => {
            let mut vec = Vec::new();
            let rx = Regex::new(&format!("refs/{}/.*", ref_prefix))?;
            for r in refs {
                let r = r?;
                let name = r.name();
                if let Some(name) = name {
                    if rx.is_match(name) {
                        vec.push(r)
                    }
                }
            }
            vec
        }
    };

    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for r in branch_refs.iter() {
        if let Some(latest_oid) = r.target() {
            let mut revwalk = repo.revwalk()?;
            revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE)?;
            revwalk.push(latest_oid)?;
            for oid in revwalk {
                let oid = oid?;
                if !seen.contains(&oid) {
                    let commit = repo.find_commit(oid)?;
                    result.push(commit.clone());
                    seen.insert(oid);
                }
            }
        }
    }

    Ok(result)
}

fn to_config(matches: ArgMatches) -> Result<Config> {
    let max_commit_diff = matches.value_of("max-commit-diff").unwrap().parse::<u32>()?;
    let first_commit_addition = matches.value_of("first-commit-add").unwrap().parse::<u32>()?;

    let since = match matches.value_of("since") {
        Some(s) => CommitTimeBound::from_str(s)?,
        None => CommitTimeBound::Always
    };
    let until = match matches.value_of("until") {
        Some(s) => CommitTimeBound::from_str(s)?,
        None => CommitTimeBound::Always
    };

    let merge_requests = matches.is_present("merge-requests");
    let display_breakdown = matches.is_present("breakdown");

    let git_repo_path = matches.value_of("REPO_PATH").unwrap();

    let aliases = match matches.values_of("email") {
        Some(vs) => {
            let vec: Vec<&str> = vs.collect();
            let results: Result<Vec<(String, String)>, anyhow::Error> = vec
                .iter()
                .try_fold(Vec::new(), |mut acc, e| {
                    let alias = parse_email_alias(e)?;
                    acc.push(alias);
                    Ok(acc)
                });
            results?
        },
        None => Vec::new()
    }.into_iter().collect::<HashMap<String, String>>();

    let branch = matches.value_of("branch").map(|b| b.to_string());

    let branch_type = match matches.value_of("branch-type") {
        None => BranchType::Local,
        Some("local") => BranchType::Local,
        Some("remote") => BranchType::Remote,
        Some(x) => return Err(anyhow!("Invalid branch type '{}'", x))
    };

    let output_format = value_t!(matches, "format", OutputFormat).unwrap();

    Ok(Config {
        max_commit_diff: Duration::minutes(max_commit_diff.into()),
        first_commit_addition: Duration::minutes(first_commit_addition.into()),
        since: since,
        until: until,
        merge_requests: merge_requests,
        git_repo_path: PathBuf::from(git_repo_path),
        email_aliases: aliases,
        branch: branch,
        branch_type: branch_type,
        output_format: output_format,
        display_breakdown: display_breakdown
    })
}

fn filter_commits<'repo>(config: &Config, commits: Vec<Commit<'repo>>) -> Vec<Commit<'repo>> {
    let since = config.since.to_date_time();
    let until = config.until.to_date_time();

    let since_local = since.map(|b| Local.from_local_datetime(&b).unwrap());
    let until_local = until.map(|b| Local.from_local_datetime(&b).unwrap());

    commits.into_iter().filter(|commit| {
        let time = commit.time();
        if let Some(bound) = since_local {
            let dt = Utc.timestamp(time.seconds(), 0);
            if dt < bound {
                return false
            }
        }
        if let Some(bound) = until_local {
            let dt = Utc.timestamp(time.seconds(), 0);
            if dt > bound {
                return false
            }
        }

        if !config.merge_requests {
            if commit.summary().map(|s| s.starts_with("Merge ")).unwrap_or(false) {
                return false
            }
        }

        true
    }).collect()
}

#[derive(Clone)]
struct CommitHours {
    email: Option<String>,
    author_name: Option<String>,
    breakdown: HashMap<String, Duration>,
    duration: Duration,
    commit_count: usize
}

fn estimate_author_time(mut commits: Vec<Commit>, email: Option<String>, max_commit_diff: &Duration, first_commit_addition: &Duration, display_breakdown: &bool) -> CommitHours {
    let author_name = commits[0].author().name().map(|n| n.to_string());

    commits.sort_by(|a, b| a.time().cmp(&b.time()));

    //let lalala = Utc.timestamp(commits[commits.len() - 3].time().seconds(), 0).format("%Y-%m-%d");
    //let lilili = Utc.timestamp(commits[commits.len() - 2].time().seconds(), 0).format("%Y-%m-%d");
    //println!("just checking two last commits");
    //println!("{}", lalala);
    //println!("{}", lilili);

    let mut coding_session_start = Utc.timestamp(commits[0].time().seconds(), 0).format("%Y-%m-%d");
    let len = commits.len() - 1;
    let all_but_last = commits.iter().enumerate().take(len);
    let mut breakdown = HashMap::new();
    breakdown.entry(coding_session_start.to_string())
        .or_insert_with(|| *first_commit_addition);

    for (i, commit) in all_but_last {
        let next_commit = commits.get(i+1).unwrap();
        let diff_seconds = next_commit.time().seconds() - commit.time().seconds();
        let dur = Duration::seconds(diff_seconds);

        if dur < *max_commit_diff {
            breakdown.entry(coding_session_start.to_string())
                .and_modify(|e| { *e = *e + dur })
                .or_insert_with(|| dur);
        } else {
            coding_session_start = Utc.timestamp(next_commit.time().seconds(), 0).format("%Y-%m-%d");
            breakdown.entry(coding_session_start.to_string())
                .and_modify(|e| { *e = *e + *first_commit_addition })
                .or_insert_with(|| *first_commit_addition);
        }
    };

    let duration = breakdown.values().fold(Duration::minutes(0), |acc, dur| {
        acc + *dur
    });

    CommitHours {
        email: email,
        author_name: author_name,
        breakdown: breakdown,
        duration: duration,
        commit_count: commits.len()
    }
}

fn estimate_author_times(config: &Config, commits: Vec<Commit>) -> Vec<CommitHours> {
    let mut no_email = Vec::new();
    let mut by_email: HashMap<String, Vec<Commit>> = HashMap::new();

    for commit in commits {
        let author_commits = {
            let author = commit.author();
            let email = author.email().map(|e| {
                match config.email_aliases.get(e) {
                    Some(alias) => alias,
                    None => e
                }
            });

            match email {
                Some(e) => {
                    by_email.entry(e.to_string()).or_insert_with(|| Vec::new())
                },
                None => {
                    &mut no_email
                }
            }
        };

        author_commits.push(commit)
    }

    let mut result = Vec::new();

    if no_email.len() > 0 {
        result.push(estimate_author_time(no_email, None, &config.max_commit_diff, &config.first_commit_addition, &config.display_breakdown));
    }

    for (email, author_commits) in by_email {
        result.push(estimate_author_time(author_commits, Some(email), &config.max_commit_diff, &config.first_commit_addition, &config.display_breakdown));
    }

    result.sort_by(|a, b| {
        let ord = b.duration.cmp(&a.duration);
        if ord != Ordering::Equal {
            return ord
        }
        b.commit_count.cmp(&a.commit_count)
    });

    result
}

fn get_totals(times: &Vec<CommitHours>) -> (f32, usize) {
    let mut total_estimated_hours = 0.0;
    let mut total_commits = 0;
    for time in times.iter() {
        let commits = time.commit_count;
        let estimated_hours = (time.duration.num_minutes() as f32) / 60.0;
        total_commits += commits;
        total_estimated_hours += estimated_hours;
    }

    (total_estimated_hours, total_commits)
}

fn print_results_stdout(times: &Vec<CommitHours>) -> Result<()> {
    let mut table = Table::new();

    let format = format::FormatBuilder::new()
        .column_separator('|')
        .borders('|')
        .separators(&[format::LinePosition::Top,
                      format::LinePosition::Bottom],
                    format::LineSeparator::new('-', '+', '+', '+'))
        .padding(1, 1)
        .build();
    table.set_format(format);

    table.set_titles(row!["Author", "Email", "Commits", "Estimated Hours"]);
    table.add_empty_row();

    for time in times.iter() {
        let author = match &time.author_name {
            Some(n) => n,
            None => ""
        };
        let email = match &time.email {
            Some(email) => email,
            None => "(none)"
        };
        let commits = time.commit_count;
        let estimated_hours = (time.duration.num_minutes() as f32) / 60.0;

        table.add_row(row![author, email, commits, estimated_hours]);
    }

    table.add_empty_row();

    let (total_estimated_hours, total_commits) = get_totals(times);
    table.add_row(row!["Total", "", total_commits, total_estimated_hours]);

    table.printstd();

    Ok(())
}

fn print_breakdown_results_stdout(times: &Vec<CommitHours>) -> Result<()> {
    let mut table = Table::new();

    let format = format::FormatBuilder::new()
        .column_separator('|')
        .borders('|')
        .separators(&[format::LinePosition::Top,
                      format::LinePosition::Bottom],
                    format::LineSeparator::new('-', '+', '+', '+'))
        .padding(1, 1)
        .build();
    table.set_format(format);

    table.set_titles(row!["Author", "Email", "Commits", "Date", "Estimated Hours"]);
    table.add_empty_row();

    for time in times.iter() {
        let author = match &time.author_name {
            Some(n) => n,
            None => ""
        };
        let email = match &time.email {
            Some(email) => email,
            None => "(none)"
        };
        let commits = time.commit_count;
        let estimated_total_hours = (time.duration.num_minutes() as f32) / 60.0;

        for (date, duration) in &(time.breakdown) {
            let date = date.to_owned();
            let work_time = duration.num_minutes() as f32 / 60.0;
            table.add_row(row!["", "", "", date, work_time]);
        }
        table.add_row(row![author, email, commits, "Total", estimated_total_hours]);
        table.add_empty_row();
    }

    table.add_empty_row();

    let (total_estimated_hours, total_commits) = get_totals(times);
    table.add_row(row!["Total", "", total_commits, "", total_estimated_hours]);

    table.printstd();

    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
struct CommitHoursJson {
    email: Option<String>,
    author_name: Option<String>,
    breakdown: Option<HashMap<String, f32>>,
    hours: f32,
    commit_count: usize
}

impl From<&CommitHours> for CommitHoursJson {
    fn from(time: &CommitHours) -> Self {
        let mut breakdown = HashMap::new();
        for (key, value) in &(time.breakdown) {
            breakdown.insert(key.to_owned(), value.num_minutes() as f32 / 60.0);
        }
        CommitHoursJson {
            email: time.email.clone(),
            author_name: time.author_name.clone(),
            breakdown: Some(breakdown),
            hours: time.duration.num_minutes() as f32 / 60.0,
            commit_count: time.commit_count,
        }
    }
}

fn print_results_json(times: &Vec<CommitHours>) -> Result<()> {
    let mut times_json = times.iter().map(CommitHoursJson::from).collect::<Vec<_>>();

    let (total_estimated_hours, total_commits) = get_totals(times);
    times_json.push(CommitHoursJson {
        email: None,
        author_name: Some(String::from("Total")),
        breakdown: None,
        hours: total_estimated_hours,
        commit_count: total_commits
    });

    let json = serde_json::to_string_pretty(&times_json)?;

    println!("{}", json);

    Ok(())
}

fn print_results(times: &Vec<CommitHours>, output_format: &OutputFormat, display_breakdown: &bool) -> Result<()> {
    match output_format {
        OutputFormat::Stdout => match *display_breakdown {
            true => print_breakdown_results_stdout(times),
            false => print_results_stdout(times)
        },
        OutputFormat::Json => print_results_json(times)
    }
}

type ExitCode = i32;

fn jikyuu(config: &Config) -> Result<ExitCode> {
    let repo = Repository::init(&config.git_repo_path)?;

    let commits = get_commits(&config.branch, config.branch_type, &repo)?;

    let filtered_commits = filter_commits(&config, commits);

    let by_author = estimate_author_times(&config, filtered_commits);

    if by_author.len() == 0 {
        match &config.branch {
            Some(b) => {
                let branch_type = match config.branch_type {
                    BranchType::Local => "local",
                    BranchType::Remote => "remote",
                };
                eprintln!("No commits found for branch '{}' ({}).", b, branch_type)
            },
            None => eprintln!("No commits found.")
        }
        Ok(1)
    } else {
        print_results(&by_author, &config.output_format, &config.display_breakdown)?;
        Ok(0)
    }
}

fn run_app() -> Result<ExitCode> {
    let matches = get_app().get_matches();

    let config = to_config(matches)?;

    jikyuu(&config)
}

fn main() {
    let exit_code = match run_app() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    };
    std::process::exit(exit_code);
}
