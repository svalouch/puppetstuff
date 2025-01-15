# Puppetstuff

Random Puppet-related code that I needed at some point in time that someone else might find useful in some way or another. It works for my use-cases, so patches are welcome, but don't expect them to be merged ;)

**Warning**: This is not "good code", it just scratches an itch I have. Use at your own risk.

`puppetstuff` operates on an git clone (or a bare repo). It parses all `Puppetfile`s in all branches whose name starts with `origin/`, i.e. the ones that are considered to be visible to your Puppet Master, so you should update your local clone before running the tool so you won't look at old data. Connecting to a remote repository (e.g. via some API), cloning or updating of repositories is out of scope.

The parser for `Puppetfile` is written to work with `g10k`, but should also work with `r10k` syntax. It extracts Forge-modules as well as Git-modules from the `Puppetfile`s, although it only cares for the Forge-modules (see below for details on that). You can use the parser to discover private module repositories, but the project is not meant to be a library, just copy &amp; paste what you need in accordance with the license.

It supports three output formats:
- A pretty and colourful UTF-8 table on the terminal (default, or `-f terminal-table`)
  - This view gets distorted if your terminal is not wide enough.
- Markdown table (`-f md`)
  - HTML-output can be created from Markdown: `puppetstuff -r ~/puppet/environment -f md forge-branches | ~/.cargo/bin/pulldown-cmark --enable-tables > my-environment.html` and embed it in something like the "Content" class from bulma.io, or slurp it up with Zola or something similar.
- Jira table, because some of us aren't allowed to have nice things to work with even in 2025 (`-f jira`)

In all of them, module names are linked to bring you to the Forge entry. The terminal output is colourized, Markdown and Jira use symbols instead of colours.

Some views can limit the information to only one specific branch. If your environment has too many to fit your screen or you only want to know how one particular branch is set up, use `-b <branchname>` (e.g. with the `forge-branches` view).

## Views

The following views are implemented:
- `forge-branches`: Outputs a table with one row per module and one column per branch (plus one for the version on the Forge), optionally limited to one branch (`-b <branchname>`).
- `forge-modules`: Outputs a table with module name and Forge version, no branch-information.

Not all formats and arguments are implemented for all of them.

## Querying the public Puppet-Forge
For modules from the Puppet-Forge, it queries these two sets of information using the API:
- latest published version, assuming strict semver-compliance
- whether the module is marked as deprecated/discontinued or not

Information is kept in a very crude cache that caches each module's information for one hour, so for up to one hour after the first run, it won't query the Forge again unless new modules are added. To clear the cache, simply remove `/tmp/asdf.json` (I told you it was crude!).

**DO NOT ABUSE THE FORGE!**

The cache is there for a reason, and may not be sufficient. Adhere to the Forge's terms of service and so on!

# Examples

## Terminal output
In reality, this is coloured:
- The cell for `puppet-systemd` / "Forge latest" has a yellow background, because there is no branch at this version (all are older).
- All the cells for the branches have a green background, because all of them are at the same version relative to each other (even though the forge may be newer). If, for example, `puppetlabs-apt` in branch `qa` was at version `9.0.0`, this cell would be yellow to indicate that it is on an older version relative to `dev`, `live` and `qa_test`.
- The "Forge latest" cells become red if the module has been marked as deprecated on the Forge, regardless of the versions in use in any of the branches.

Also, the "Module-Name" column is clickable if your terminal supports hyperlinks. It may look awful if it does not.

Branches without a `Puppetfile` are not shown, but a warning is printed to highlight the issue.

```
$ puppetstuff -r ~/puppet/work/environment forge-branches
╭─────────────────────────────┬──────────────┬────────┬────────┬────────┬─────────╮
│ Module-Name                 ┆ Forge latest ┆ dev    ┆ live   ┆ qa     ┆ qa_test │
╞═════════════════════════════╪══════════════╪════════╪════════╪════════╪═════════╡
│ puppet-rsyslog              ┆ 7.1.0        ┆ 7.1.0  ┆ 7.1.0  ┆ 7.1.0  ┆ 7.1.0   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ puppet-systemd              ┆ 8.1.0        ┆ 8.0.0  ┆ 8.0.0  ┆ 8.0.0  ┆ 8.0.0   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ puppetlabs-stdlib           ┆ 9.7.0        ┆ 9.7.0  ┆ 9.7.0  ┆ 9.7.0  ┆ 9.7.0   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ saz-locales                 ┆ 4.0.2        ┆ 4.0.2  ┆ 4.0.2  ┆ 4.0.2  ┆ 4.0.2   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ saz-timezone                ┆ 7.0.0        ┆ 7.0.0  ┆ 7.0.0  ┆ 7.0.0  ┆ 7.0.0   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ stm-debconf                 ┆ 6.1.0        ┆ 6.1.0  ┆ 6.1.0  ┆ 6.1.0  ┆ 6.1.0   │
╰─────────────────────────────┴──────────────┴────────┴────────┴────────┴─────────╯
```

## Markdown
Instead of yellow like in the terminal example, ⏰ is used. A 🔥 is used instead of red (deprecated module).

```md
$ puppetstuff -r ~/puppet/work/environment -f md forge-branches
|Module-Name|Forge latest|dev|live|qa|qa_test|
|: - |: - |: - |: - |: - |: - |
|[puppet-rsyslog](https://forge.puppet.com/modules/puppet/rsyslog)|`7.1.0`|`7.1.0`|`7.1.0`|`7.1.0`|`7.1.0`|
|[puppet-systemd](https://forge.puppet.com/modules/puppet/systemd)|`8.1.0` ⏰|`8.0.0`|`8.0.0`|`8.0.0`|`8.0.0`|
|[puppetlabs-stdlib](https://forge.puppet.com/modules/puppetlabs/stdlib)|`9.7.0`|`9.7.0`|`9.7.0`|`9.7.0`|`9.7.0`|
|[saz-locales](https://forge.puppet.com/modules/saz/locales)|`4.0.2`|`4.0.2`|`4.0.2`|`4.0.2`|`4.0.2`|
|[saz-timezone](https://forge.puppet.com/modules/saz/timezone)|`7.0.0`|`7.0.0`|`7.0.0`|`7.0.0`|`7.0.0`|
|[stm-debconf](https://forge.puppet.com/modules/stm/debconf)|`6.1.0`|`6.1.0`|`6.1.0`|`6.1.0`|`6.1.0`|
```

## Jira
I feel your pain!

A red X is used to denote deprecated and a yellow exclamation mark denotes outdated versions.

Also, this will likely get butchered if someone ever uses the visual editor (even if just to check the output before saving). It is what it is.

```
$ puppetstuff -r ~/puppet/work/environment -f jira forge-branches
||{{Module-Name}}||{{Forge latest}}||{{dev}}||{{live}}||{{qa}}||{{qa_test}}||
|[puppet-rsyslog|https://forge.puppet.com/modules/puppet/rsyslog]|{{7.1.0}}|{{7.1.0}}|{{7.1.0}}|{{7.1.0}}|{{7.1.0}}|
|[puppet-systemd|https://forge.puppet.com/modules/puppet/systemd]|{{8.1.0}} (!)|{{8.0.0}}|{{8.0.0}}|{{8.0.0}}|{{8.0.0}}|
|[puppetlabs-stdlib|https://forge.puppet.com/modules/puppetlabs/stdlib]|{{9.7.0}}|{{9.7.0}}|{{9.7.0}}|{{9.7.0}}|{{9.7.0}}|
|[saz-locales|https://forge.puppet.com/modules/saz/locales]|{{4.0.2}}|{{4.0.2}}|{{4.0.2}}|{{4.0.2}}|{{4.0.2}}|
|[saz-timezone|https://forge.puppet.com/modules/saz/timezone]|{{7.0.0}}|{{7.0.0}}|{{7.0.0}}|{{7.0.0}}|{{7.0.0}}|
|[stm-debconf|https://forge.puppet.com/modules/stm/debconf]|{{6.1.0}}|{{6.1.0}}|{{6.1.0}}|{{6.1.0}}|{{6.1.0}}|
```

I suggest to use a script to replace a specific comment (or the entire description) with this output each time you update it. This will save you a lot of headaches if some people insist on the visual editor.

