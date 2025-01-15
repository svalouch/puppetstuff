// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, Cell};
use comfy_table::{Color, Table};
use git2::BranchType;
use semver::Version;
use std::collections::{HashMap, HashSet};
use std::process;
use tracing::{debug, error, span, trace, warn};

mod forge;
mod models;
mod puppetfile;

#[derive(Subcommand)]
enum View {
    /// Show latest releases for forge-crates
    ForgeLatest,
    /// Show releases across branches
    ForgeBranches,
    /// Show deprecated modules
    ForgeDeprecated,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Jira-Markup
    Jira,
    /// MarkDown
    Md,
    /// Pretty for the terminal
    TerminalTable,
}
impl ToString for OutputFormat {
    fn to_string(&self) -> String {
        match self {
            OutputFormat::Jira => "jira".into(),
            OutputFormat::Md => "markdown".into(),
            OutputFormat::TerminalTable => "terminal-table".into(),
        }
    }
}

#[derive(Parser)]
struct Cli {
    /// Clone to work on, omit for current directory
    #[arg(short, long)]
    repo: Option<String>,
    /// Output format
    #[arg(short, long, default_value_t = OutputFormat::TerminalTable)]
    format: OutputFormat,
    /// Show only this branch in views that support it
    #[arg(short, long)]
    branch: Option<String>,
    #[command(subcommand)]
    view: View,
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();

    let repo_path = match args.repo {
        Some(p) => shellexpand::tilde(&p).into_owned(),
        None => String::from("."),
    };

    let mut api = forge::ForgeApi::new(Some("/tmp/asdf.json".to_string()));

    let (branch_modules, forge_names) = parse_git_repo(&repo_path);

    // list of all module names we discovered. Some may not be present in a particular branch. Sort
    // them for consistency.
    let forge_names = {
        let mut fname: Vec<String> = forge_names.into_iter().collect();
        fname.sort();
        fname
    };

    let branch_names = {
        let mut bname: Vec<String> = branch_modules
            .clone()
            .into_iter()
            .map(|bm| bm.name.clone())
            .collect();
        // required for consistent output
        bname.sort();
        bname
    };
    // format it once for easier querying later
    let selected_branch = args
        .branch
        .and_then(|bname| Some(format!("origin/{bname}")));
    // if the user wanted a specific branch and we can't find it, we exit with a helpful message
    if let Some(ref selected_branch) = selected_branch {
        if !branch_names.contains(&selected_branch) {
            eprintln!("Sorry, the selected branch is not known. Branches to choose from:");
            for branch in branch_names {
                eprintln!("\t{}", branch.replace("origin/", ""));
            }
            process::exit(1);
        }
    }

    match args.view {
        View::ForgeLatest => {
            match args.format {
                OutputFormat::TerminalTable => {
                    let mut module_overview_table = Table::new();
                    module_overview_table
                        .load_preset(UTF8_FULL)
                        .apply_modifier(UTF8_ROUND_CORNERS)
                        .set_header(vec!["Name", "Latest"]);
                    for name in forge_names.into_iter() {
                        let title = Cell::new(forge_module_console_hyperlink(
                            &name.replacen("-", "/", 1),
                            &name,
                        ))
                        .add_attribute(comfy_table::Attribute::Underlined);

                        let vers = Cell::new(api.get_version(&name).unwrap().to_string());
                        let vers = match api.is_deprecated(&name).unwrap() {
                            true => vers.bg(Color::Red),
                            false => vers,
                        };

                        module_overview_table.add_row([title, vers]);
                    }
                    println!("{module_overview_table}");
                }
                _ => (),
            };
        }
        View::ForgeBranches => {
            #[derive(Debug)]
            struct ModuleRow {
                name: String,
                forge_version: Version,
                forge_deprecated: bool,
                max_in_use_version: Version,
                branch_versions: HashMap<String, Option<Version>>,
            }
            let mut module_rows: Vec<ModuleRow> = vec![];

            // find info about a specific module (one at a time) to figure out if and with what
            // version it is used in a specific branch, or mark it as not used for that branch
            for mod_name in forge_names {
                let sp = span!(tracing::Level::DEBUG, "forge-mod-loop", mod_name = mod_name);
                let _sp = sp.enter();
                let mut row = ModuleRow {
                    name: mod_name.clone(),
                    forge_version: api.get_version(&mod_name).unwrap(),
                    forge_deprecated: api.is_deprecated(&mod_name).unwrap(),
                    max_in_use_version: Version::new(0, 0, 0),
                    branch_versions: HashMap::new(),
                };

                for branch in &branch_modules {
                    debug!("Branch {}", branch.name);
                    for branch_module in branch.modules.clone() {
                        // debug!("Branch module: {branch_module:?}");
                        match branch_module {
                            models::Module::Forge(name, version) => {
                                if name == mod_name {
                                    row.max_in_use_version = std::cmp::max(
                                        version.clone(),
                                        row.max_in_use_version.clone(),
                                    );
                                    row.branch_versions
                                        .insert(branch.name.clone(), Some(version.clone()));
                                    break;
                                }
                            }
                            _ => (),
                        };
                    }
                }
                module_rows.push(row);
            }

            let mut table = Table::new(); // terminal-table
            let mut fmt_rows: Vec<String> = vec![]; // jira+md

            let mut table_header: Vec<String> = vec!["Module-Name".into(), "Forge latest".into()];
            table_header.extend(match selected_branch {
                Some(ref bname) => vec![bname.clone().replace("origin/", "")],
                None => branch_names
                    .clone()
                    .into_iter()
                    .map(|bn| bn.replace("origin/", ""))
                    .collect::<Vec<String>>(),
            });

            // build the table's header
            match args.format {
                OutputFormat::TerminalTable => {
                    table
                        .load_preset(UTF8_FULL)
                        .apply_modifier(UTF8_ROUND_CORNERS)
                        .set_header(table_header);
                }
                OutputFormat::Jira => {
                    fmt_rows.push(format!("||{{{{{}}}}}||", table_header.join("}}||{{")));
                }
                OutputFormat::Md => {
                    fmt_rows.push(format!("|{}|", table_header.join("|")));
                    // create the simplest-possible header with alignment-specification:
                    fmt_rows.push(format!(
                        "|{}|",
                        (0..table_header.len())
                            .map(|_| ": - ")
                            .collect::<Vec<&str>>()
                            .join("|")
                    ));
                }
            };

            for mod_row in module_rows {
                trace!("{mod_row:?}");
                let mut cell_row: Vec<Cell> = vec![];
                let mut fmt_row: Vec<String> = vec![];

                // Format the module-name+link and the forge version with
                // freshness-indicators:
                match args.format {
                    OutputFormat::TerminalTable => {
                        cell_row.push(
                            Cell::new(forge_module_console_hyperlink(
                                &mod_row.name.replacen("-", "/", 1),
                                &mod_row.name,
                            ))
                            .add_attribute(comfy_table::Attribute::Underlined),
                        );

                        let forge_cell = Cell::new(&mod_row.forge_version);
                        let forge_cell = if mod_row.forge_deprecated {
                            forge_cell.bg(Color::Red).fg(Color::Black)
                        } else if mod_row.forge_version > mod_row.max_in_use_version {
                            forge_cell.bg(Color::DarkYellow).fg(Color::Black)
                        } else {
                            forge_cell
                        };
                        cell_row.push(forge_cell);
                    }
                    OutputFormat::Jira => {
                        fmt_row.push(format!(
                            "[{}|https://forge.puppet.com/modules/{}]",
                            &mod_row.name,
                            &mod_row.name.replacen("-", "/", 1)
                        ));
                        if mod_row.forge_deprecated {
                            fmt_row.push(format!("{{{{{}}}}} (x)", mod_row.forge_version));
                        } else if mod_row.forge_version > mod_row.max_in_use_version {
                            fmt_row.push(format!("{{{{{}}}}} (!)", mod_row.forge_version));
                        } else {
                            fmt_row.push(format!("{{{{{}}}}}", mod_row.forge_version));
                        }
                    }
                    OutputFormat::Md => {
                        fmt_row.push(format!(
                            "[{}](https://forge.puppet.com/modules/{})",
                            &mod_row.name,
                            &mod_row.name.replacen("-", "/", 1)
                        ));
                        if mod_row.forge_deprecated {
                            fmt_row.push(format!("`{}` 🔥", mod_row.forge_version));
                        } else if mod_row.forge_version > mod_row.max_in_use_version {
                            fmt_row.push(format!("`{}` ⏰", mod_row.forge_version));
                        } else {
                            fmt_row.push(format!("`{}`", mod_row.forge_version));
                        }
                    }
                };

                // iterate by branch to be consistent with the headers
                for branch_name in branch_names.iter() {
                    if let Some(ref bname) = selected_branch {
                        if bname != branch_name {
                            debug!("Selected branch: {bname} → skipping branch {branch_name}");
                            continue;
                        }
                    }
                    debug!("branch {branch_name}");
                    let mut found_in_branch = false;
                    for (mod_branch_name, branch_version) in mod_row.branch_versions.iter() {
                        if branch_name == mod_branch_name {
                            if let Some(branch_version) = branch_version {
                                if branch_version < &mod_row.max_in_use_version {
                                    match args.format {
                                        OutputFormat::TerminalTable => {
                                            cell_row.push(
                                                Cell::new(branch_version.to_string())
                                                    .bg(Color::DarkYellow)
                                                    .fg(Color::Black),
                                            );
                                        }
                                        OutputFormat::Jira => {
                                            fmt_row.push(format!("{{{{{branch_version}}}}} (!)"));
                                        }
                                        OutputFormat::Md => {
                                            fmt_row.push(format!("`{branch_version}` ⏰"));
                                        }
                                    };
                                } else if branch_version > &mod_row.max_in_use_version {
                                    match args.format {
                                        OutputFormat::TerminalTable => {
                                            cell_row.push(
                                                Cell::new(branch_version.to_string())
                                                    .bg(Color::Yellow)
                                                    .fg(Color::Black),
                                            );
                                        }
                                        OutputFormat::Jira => {
                                            fmt_row.push(format!("{{{{{branch_version}}}}} (x)"));
                                        }
                                        OutputFormat::Md => {
                                            fmt_row.push(format!("`{branch_version}` 🔥"));
                                        }
                                    };
                                } else {
                                    match args.format {
                                        OutputFormat::TerminalTable => {
                                            cell_row.push(
                                                Cell::new(branch_version.to_string())
                                                    .bg(Color::DarkGreen)
                                                    .fg(Color::Black),
                                            );
                                        }
                                        OutputFormat::Jira => {
                                            fmt_row.push(format!("{{{{{branch_version}}}}}"));
                                        }
                                        OutputFormat::Md => {
                                            fmt_row.push(format!("`{branch_version}`"));
                                        }
                                    };
                                }
                            } else {
                                if args.format == OutputFormat::TerminalTable {
                                    cell_row.push(Cell::new(""));
                                } else {
                                    fmt_row.push(" ".to_string());
                                }
                            }
                            found_in_branch = true;
                            break;
                        }
                    }
                    if !found_in_branch {
                        if args.format == OutputFormat::TerminalTable {
                            cell_row.push(Cell::new(""));
                        } else {
                            fmt_row.push(" ".to_string());
                        }
                    }
                }
                // assemble the row and add it to the table
                if args.format == OutputFormat::TerminalTable {
                    table.add_row(cell_row);
                } else {
                    fmt_rows.push(format!("|{}|", fmt_row.join("|")));
                }
            }
            if args.format == OutputFormat::TerminalTable {
                println!("{table}");
            } else {
                for row in fmt_rows {
                    println!("{row}");
                }
            }
        }
        View::ForgeDeprecated => {}
    };

    api.store_cache("/tmp/asdf.json");
}

fn parse_git_repo(repo_path: &str) -> (Vec<models::BranchMeta>, HashSet<String>) {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => {
            error!("Error opening repo: {e}");
            std::process::exit(1);
        }
    };
    let mut branch_modules = vec![];
    let mut forge_names = HashSet::new();

    let branches = repo.branches(Some(BranchType::Remote)).unwrap();
    for (branch, _btype) in branches.into_iter().filter_map(|b| b.ok()) {
        let name = match branch.name() {
            Ok(n) => match n {
                Some(n) => n.to_owned(),
                None => {
                    warn!("Branch name contains invalid characters, skipping");
                    continue;
                }
            },
            Err(e) => {
                warn!("Could not retrieve branch name: {e}");
                continue;
            }
        };
        if !name.starts_with("origin") {
            debug!("Skipping branch {name}");
            continue;
        }

        let reference = branch.into_reference();

        if reference.kind() == Some(git2::ReferenceType::Direct) {
            if let Some(oid) = reference.target() {
                debug!("{:?} {oid:?}", reference.name());

                let commit = repo.find_commit(oid).unwrap();
                // let author_date = commit.author().when();
                // let commit_date = commit.time();
                // let author = format!(
                //     "{} <{}>",
                //     commit.author().name().unwrap_or("Unknown"),
                //     commit.author().email().unwrap_or("unknown@unknown")
                // );

                let tree = commit.tree().unwrap();
                let pf_entry = match tree.get_name("Puppetfile") {
                    Some(te) => te,
                    None => {
                        warn!("Could not find 'Puppetfile' in the root tree of branch {name}");
                        continue;
                    }
                };
                let pf_blob = match repo.find_blob(pf_entry.id()) {
                    Ok(b) => b.content().to_owned(),
                    Err(e) => {
                        warn!(
                            "Could not get blob for tree entry 'Puppetfile' in branch {name}: {e}"
                        );
                        continue;
                    }
                };
                let pf_blob = std::str::from_utf8(&pf_blob).unwrap();

                let modules = puppetfile::parse_puppetfile(pf_blob);
                forge_names.extend(modules.iter().filter_map(|module| match module {
                    models::Module::Forge(name, _) => Some(name.to_owned()),
                    _ => None,
                }));

                branch_modules.push(models::BranchMeta {
                    name,
                    // oid,
                    // author_date,
                    // commit_date,
                    // author,
                    modules,
                });
            }
        }
    }
    (branch_modules, forge_names)
}

fn forge_module_console_hyperlink(href: &str, title: &str) -> String {
    format!("\x1B]8;;https://forge.puppet.com/modules/{href}\x1B\\{title}\x1B]8;;\x1B\\",)
}
