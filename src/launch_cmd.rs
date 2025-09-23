use std::{collections::HashMap, path::Path, process::{Child, Command}};

pub struct LaunchCommand {
    cmd: Command,
    ctx: HashMap<&'static str, String>,
    args: Vec<String>
}

impl LaunchCommand {
    pub fn new(
        launch_dir: &Path,
        java_path: Option<&String>,
        java_args: Option<&Vec<String>>,
        java_env: Option<&HashMap<String, String>>
    ) -> Self {
        // use java override path from instance manifest, or default to "java" in PATH
        let mut cmd = if let Some(path) = java_path {
            Command::new(path)
        } else {
            Command::new("java")
        };

        if let Some(args) = java_args {
            cmd.args(args);
        }

        if let Some(vars) = java_env {
            cmd.envs(vars);
        }

        // set current directory for log output
        cmd.current_dir(launch_dir);

        Self {
            cmd: cmd,
            ctx: HashMap::new(),
            args: Vec::new()
        }
    }

    pub fn arg_ctx<S: Into<String>>(&mut self, key: &'static str, val: S) -> &mut Self {
        self.ctx.insert(key, val.into());
        self
    }

    pub fn arg<S: Into<String>>(&mut self, val: S) -> &mut Self {
        self.args.push(val.into());
        self
    }

    pub fn args<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator, I::Item: Into<String>
    {
        iter.into_iter().for_each(|v| self.args.push(v.into()));
        self
    }

    pub fn spawn(&mut self) -> std::io::Result<Child> {
        for arg in &self.args {
            self.cmd.arg(
                shellexpand::env_with_context_no_errors(
                    &arg,
                    |var:&str| self.ctx.get(var)
                ).to_string()
            );
        }

        self.cmd.spawn()
    }
}
