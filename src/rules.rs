use std::env::consts;
use crate::json::{GameLibraryRule, OsProperties};

pub fn match_rules(rules: &Vec<GameLibraryRule>) -> bool {
    _match_rules(
        rules,
        match consts::OS {
            // json uses "osx" instead of "macos" for os name
            "macos" => "osx",
            os => os
        },
        "1.0", // FIXME add OS version
        consts::ARCH
    )
}

fn _match_rules(rules: &Vec<GameLibraryRule>, host_os: &str, host_version: &str, host_arch: &str) -> bool {
    let mut result = false;

    for rule in rules {
        if rule.action == "allow" {
            result = true;

            // when allow block has OS properties match and return now
            // based on inspecting data, this appears to be desired
            if let Some(os) = &rule.os {
                return _match_os_properties(&os, host_os, host_version, host_arch);
            }
        }

        if rule.action == "disallow" {
            if let Some(os) = &rule.os {
                if _match_os_properties(&os, host_os, host_version, host_arch) {
                    return false;
                }
            }
        }
    }

    result
}

fn _match_os_properties(os: &OsProperties, host_os: &str, host_version: &str, host_arch: &str) -> bool {
    os.name.as_ref().map_or(true, |v| v == host_os) &&
    // is it worth it to add os_info and regex crates just for this?
    // os.version.as_ref().map_or(true, |v| v == ) &&
    os.arch.as_ref().map_or(true, |v| v == host_arch)
}

#[cfg(test)]
mod tests {
    use super::_match_rules;
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

        assert_eq!(_match_rules(&rules, "linux", "", "x86_64"), true);
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

        assert_eq!(_match_rules(&rules, "windows", "", "x86_64"), false);
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

        assert_eq!(_match_rules(&rules, "linux", "", "x86_64"), true);
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

        assert_eq!(_match_rules(&rules, "osx", "", "x86_64"), false);
    }
}
