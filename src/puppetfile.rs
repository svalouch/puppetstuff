// SPDX-License-Identifier: GPL-3.0-only

use regex::Regex;
use semver::Version;
use std::sync::LazyLock;
use tracing::{debug, error, trace, warn};

use crate::models::*;

/// Parse a `Puppetfile` content into a list of modules, assuming it is compliant with `g10k`.
pub fn parse_puppetfile(content: &str) -> Vec<Module> {
    // Matches a normal forge line like `mod "puppet/dance", "1.0.0"`
    static FORGE_MODULE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^\s*(?:mod)\s+['\"](?P<name>[^'\"]+[-/][^'\"]+)['\"],\s+['\"](?P<version>.*)['\"]"#,
        )
        .unwrap()
    });
    // Matches a line like `mod "mymodule",`
    static GIT_MODULE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^\s*(?:mod)\s+['\"](?P<name>[^'\"]+)['\"]\s*,$"#).unwrap());
    static GIT_ATTRIBUTE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\s*:(?P<name>git|commit|tag|branch|ref|link|fallback)\s*=>\s*['\"]?(?P<value>[^'\"]+)['\"']?$"#).unwrap()
    });

    let mut modules: Vec<Module> = vec![];
    let mut current_module = None;
    for line in content
        .split("\n")
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("#"))
    {
        // q&d: get everything before the # symbol (breaks if used in URLs, but oh well)
        let line = match line.split_once('#') {
            Some((l, _)) => l.trim(),
            None => line, // line did not contain a comment
        };
        trace!("{line}");

        if let Some(caps) = FORGE_MODULE_RE.captures(line) {
            if let Some(c_m) = current_module {
                debug!("Forge module found → previously worked-on module is complete");
                modules.push(c_m);
                current_module = None;
            }
            let name = caps
                .name("name")
                .unwrap()
                .as_str()
                .to_string()
                // forge modules are "{author}-{name}" but r10k/g10k accepts a "{author}/{name}" as well. Canonicalize it here:
                .replace("/", "-");
            let version = Version::parse(&caps.name("version").unwrap().as_str()).unwrap();
            debug!("Forge module: {} {}", name, version);
            modules.push(Module::Forge(name, version));
        } else if let Some(caps) = GIT_MODULE_RE.captures(line) {
            if let Some(c_m) = current_module {
                debug!("Git module found → previously worked-on module is complete");
                modules.push(c_m.clone());
            }
            debug!("Git module: {}", &caps[1]);
            current_module = Some(Module::Git(
                caps.name("name").unwrap().as_str().to_string(),
                GitSpec {
                    url: None,
                    reference: GitRef::Head,
                    fallback: None,
                    link: false,
                },
            ));
        } else if let Some(caps) = GIT_ATTRIBUTE_RE.captures(line) {
            if let Some(ref mut c_m) = current_module {
                let name = caps.name("name").unwrap().as_str();
                let value = caps.name("value").unwrap().as_str();
                debug!("Git attribute {name}");
                match c_m {
                    Module::Forge(_, _) => {
                        error!("Git attribute but parsing forge mod");
                        break;
                    }
                    Module::Git(_, spec) => {
                        match name {
                            "git" => {
                                debug!("Found url: {value}");
                                spec.url = Some(value.to_string());
                            }
                            "tag" => {
                                debug!("Found tag: {value}");
                                spec.reference = GitRef::Tag(value.to_string());
                            }
                            "branch" => {
                                debug!("Found branch: {value}");
                                spec.reference = GitRef::Branch(value.to_string());
                            }
                            "commit" => {
                                debug!("Found commit hash: {value}");
                                spec.reference = GitRef::Commit(value.to_string());
                            }
                            "fallback" => {
                                debug!("Found fallback branch name: {value}");
                                spec.fallback = Some(value.to_string());
                            }
                            "link" => {
                                debug!("Found link setting");
                                spec.link = true;
                            }
                            other => {
                                warn!("Found unknown git attribute: {other} => {value}");
                                break;
                            }
                        };
                    }
                };
            } else {
                error!("Hit a git attribute line but not parsing a module!");
                break;
            }
        }
    }
    if let Some(c_m) = current_module {
        debug!("End of file → previously worked-on module is complete");
        modules.push(c_m);
    }

    modules
}
