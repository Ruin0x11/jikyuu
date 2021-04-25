# jikyuu

A tool to estimate the amount of time spent working on a Git repository.

It is a direct port of [git-hours](https://github.com/kimmobrunfeldt/git-hours), written in Node, because the code was many years out of date and no longer builds.

Note that the information provided is only a rough estimate.

## Example

``` sh
git clone https://github.com/twbs/bootstrap
cd bootstrap
jikyuu

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

## Algorithm

See the [How it works](https://github.com/kimmobrunfeldt/git-hours#how-it-works) section of the git-hours README.

## Usage

Run the following to analyze the provided Git repository.

```sh
jikyuu /path/to/git/repo/
```

Extended usage:

``` sh
USAGE:
    jikyuu [FLAGS] [OPTIONS] <PATH>

FLAGS:
    -h, --help              Prints help information
    -m, --merge-requests    Include merge requests into calculation
    -V, --version           Prints version information

OPTIONS:
    -b, --branch <branch>                                                Analyze only data on the specified branch
    -t, --branch-type <local|remote>
            Type of branch that `branch` refers to. `local` means refs/heads/, `remote` means refs/remotes/.

    -e, --email <OTHER_EMAIL=MAIN_EMAIL>...                              Associate an email with a contributor
    -a, --first-commit-add <MINUTES>
            How many minutes first commit of session should add to total [default: 120]

    -d, --max-commit-diff <MINUTES>
            Maximum difference in minutes between commits counted to one session [default: 120]

    -s, --since <always|today|yesterday|thisweek|lastweek|YYYY-mm-dd>
            Analyze data since certain date [default: always]

    -u, --until <always|today|yesterday|thisweek|lastweek|YYYY-mm-dd>
            Analyze data until certain date [default: always]

```

## License

MIT.
