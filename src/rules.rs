/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2023 Josh Kropf <josh@slashdev.ca>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::json::{GameLibraryRule, OsProperties, GameArgRule};

pub trait RulesMatch {
    fn matches(&self) -> bool;
}

impl RulesMatch for Vec<GameLibraryRule> {
    fn matches(&self) -> bool {
        _match_lib_rules(self, &RulesContext::new())
    }
}

impl RulesMatch for Vec<GameArgRule> {
    fn matches(&self) -> bool {
        _match_arg_rules(self, &RulesContext::new())
    }
}

#[allow(dead_code)]
struct RulesContext {
    host_os: &'static str,
    host_version: &'static str,
    host_arch: &'static str
}

impl RulesContext {
    fn new() -> Self {
        RulesContext {
            host_os: crate::env::get_host_os(),
            host_version: "1.0", // FIXME add OS version
            host_arch: std::env::consts::ARCH
        }
    }
}

fn _match_lib_rules(rules: &Vec<GameLibraryRule>, ctx: &RulesContext) -> bool {
    let mut result = false;

    for rule in rules {
        if rule.action == "allow" {
            result = true;

            // when allow block has OS properties match and return now
            // based on inspecting data, this appears to be desired
            if let Some(os) = &rule.os {
                return _match_os_properties(os, ctx);
            }
        }

        if rule.action == "disallow" {
            if let Some(os) = &rule.os {
                if _match_os_properties(os, ctx) {
                    return false;
                }
            }
        }
    }

    result
}

fn _match_arg_rules(rules: &Vec<GameArgRule>, ctx: &RulesContext) -> bool {
    for rule in rules {
        if rule.action == "allow" {
            if let Some(_features) = &rule.features {
                // FIXME not implemented
                return false;
            }

            if let Some(os) = &rule.os {
                return _match_os_properties(os, ctx);
            }
        }
    }

    // rules "match" when rules list is empty
    true
}

fn _match_os_properties(os: &OsProperties, ctx: &RulesContext) -> bool {
    os.name.as_ref().map_or(true, |v| v == ctx.host_os) &&
    // FIXME is it worth it to add os_info and regex crates just for this?
    // os.version.as_ref().map_or(true, |v| v == ) &&
    os.arch.as_ref().map_or(true, |v| v == ctx.host_arch)
}

#[cfg(test)]
mod tests {
    use super::{_match_lib_rules, RulesContext};
    use crate::json::{GameLibraryRule, OsProperties};

    #[test]
    fn basic_allow_true() {
        let rules = vec![
            GameLibraryRule {
                action: "allow".to_string(),
                os: Some(OsProperties {
                    name: Some("linux".to_string()),
                    version: None, arch: None
                })
            }
        ];
        let ctx = RulesContext {
            host_os: "linux",
            host_version: "",
            host_arch: "x86_64"
        };

        assert_eq!(_match_lib_rules(&rules, &ctx), true);
    }

    #[test]
    fn basic_allow_false() {
        let rules = vec![
            GameLibraryRule {
                action: "allow".to_string(),
                os: Some(OsProperties {
                    name: Some("linux".to_string()),
                    version: None, arch: None
                })
            }
        ];
        let ctx = RulesContext {
            host_os: "windows",
            host_version: "",
            host_arch: "x86_64"
        };

        assert_eq!(_match_lib_rules(&rules, &ctx), false);
    }

    #[test]
    fn disallow_true() {
        let rules = vec![
            GameLibraryRule {
                action: "allow".to_string(),
                os: None
            },
            GameLibraryRule {
                action: "disallow".to_string(),
                os: Some(OsProperties {
                    name: Some("osx".to_string()),
                    version: None, arch: None
                })
            }
        ];
        let ctx = RulesContext {
            host_os: "linux",
            host_version: "",
            host_arch: "x86_64"
        };

        assert_eq!(_match_lib_rules(&rules, &ctx), true);
    }

    #[test]
    fn disallow_false() {
        let rules = vec![
            GameLibraryRule {
                action: "allow".to_string(),
                os: None
            },
            GameLibraryRule {
                action: "disallow".to_string(),
                os: Some(OsProperties {
                    name: Some("osx".to_string()),
                    version: None, arch: None
                })
            }
        ];
        let ctx = RulesContext {
            host_os: "osx",
            host_version: "",
            host_arch: "x86_64"
        };

        assert_eq!(_match_lib_rules(&rules, &ctx), false);
    }
}
