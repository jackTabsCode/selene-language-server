use zed_extension_api as zed;

struct Extension {}

impl zed::Extension for Extension {
    fn new() -> Self {
        Self {}
    }

    fn language_server_command(
        &mut self,
        _: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let path = worktree
            .which("selene-language-server")
            .ok_or_else(|| "selene-language-server is not installed".to_string())?;

        Ok(zed::Command {
            command: path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(Extension);
