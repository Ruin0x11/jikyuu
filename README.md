# jikyuu (時給)

A tool to estimate the amount of time spent working on a Git repository.

It is a direct port of [git-hours](https://github.com/kimmobrunfeldt/git-hours), written in Node.js, because the code was many years out of date and no longer builds.

Note that the information provided is only a rough estimate.

## Example

``` sh
git clone https://github.com/twbs/bootstrap
cd bootstrap
jikyuu

```

```
+----------------+-------------------------+---------+-----------------+
| Author         | Email                   | Commits | Estimated Hours |
|                |                         |         |                 |
| Mark Otto      | markdotto@gmail.com     | 2902    | 1808.9833       |
| Mark Otto      | otto@github.com         | 2516    | 1709.4          |
| XhmikosR       | xhmikosr@gmail.com      | 1431    | 1612.4667       |
| Chris Rebert   | code@rebertia.com       | 945     | 1019.3          |
| Jacob Thornton | jacobthornton@gmail.com | 826     | 740.35          |
| Mark Otto      | markotto@twitter.com    | 858     | 663.7167        |
| <...>          |                         |         |                 |
|                |                         |         |                 |
| Total          |                         | 16639   | 15041.153       |
+----------------+-------------------------+---------+-----------------+
```

You can associate an author that has used multiple emails in the commit logs with the `--email` (`-e`) option.

``` sh
jikyuu -e markotto@twitter.com=markdotto@gmail.com \
       -e otto@github.com=markdotto@gmail.com \
       -e markd.otto@gmail.com=markdotto@gmail.com \
       -e mark.otto@twitter.com=markdotto@gmail.com

```

```
+-----------------+---------------------------+---------+-----------------+
| Author          | Email                     | Commits | Estimated Hours |
|                 |                           |         |                 |
| Mark Otto       | markdotto@gmail.com       | 6880    | 4662.817        |
| XhmikosR        | xhmikosr@gmail.com        | 1431    | 1612.4667       |
| Chris Rebert    | code@rebertia.com         | 945     | 1019.3          |
| Jacob Thornton  | jacobthornton@gmail.com   | 826     | 740.35          |
| Martijn Cuppens | martijn.cuppens@gmail.com | 361     | 508.5           |
| <...>           |                           |         |                 |
+-----------------+---------------------------+---------+-----------------+
```

Use `--format json` (`-f`) to output the data as a JSON array.

```json5
[
  {
    "email": "markdotto@gmail.com",
    "author_name": "Mark Otto",
    "hours": 4662.817,
    "commit_count": 6880
  },
  {
    "email": "xhmikosr@gmail.com",
    "author_name": "XhmikosR",
    "hours": 1612.4667,
    "commit_count": 1431
  },

  // ...

  {
    "email": null,
    "author_name": "Total",
    "hours": 14826.803,
    "commit_count": 16639
  }
]
```

## Algorithm

See the [How it works](https://github.com/kimmobrunfeldt/git-hours#how-it-works) section of the git-hours README.

## Usage

Run the following command to estimate the time spent for the provided Git repository.

```sh
jikyuu /path/to/git/repo/
```

The path must point to the root of the Git repo, not any subdirectories inside of it.

Extended usage:

```
USAGE:
    jikyuu [FLAGS] [OPTIONS] <REPO_PATH>

FLAGS:
    -h, --help              Prints help information
    -m, --merge-requests    Include merge requests into calculation
    -V, --version           Prints version information

OPTIONS:
    -b, --branch <branch>                                                Analyze only data on the specified branch
    -t, --branch-type <local|remote>
            Type of branch that `branch` refers to. `local` means refs/heads/, `remote` means refs/remotes/.

    -e, --email <OTHER_EMAIL=MAIN_EMAIL>...
            Associate all commits that have a secondary email with a primary email

    -a, --first-commit-add <MINUTES>
            How many minutes first commit of session should add to total [default: 120]

    -f, --format <format>
             [default: stdout]  [possible values: Stdout, Json]

    -d, --max-commit-diff <MINUTES>
            Maximum difference in minutes between commits counted to one session [default: 120]

    -s, --since <always|today|yesterday|thisweek|lastweek|YYYY-mm-dd>
            Analyze data since certain date [default: always]

    -u, --until <always|today|yesterday|thisweek|lastweek|YYYY-mm-dd>
            Analyze data until certain date [default: always]


ARGS:
    <REPO_PATH>    Root path of the Git repository to analyze.
```

## License

MIT.
