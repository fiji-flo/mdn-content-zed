use std::{fs, iter::once};

use zed_extension_api::{
    self as zed, settings::LspSettings, Architecture, Command, LanguageServerId, Os, Result,
    Worktree,
};

pub struct RariBinary {
    path: String,
    args: Option<Vec<String>>,
    environment: Option<Vec<(String, String)>>,
}

pub struct MDN {
    binary_path: Option<String>,
}

impl MDN {
    pub fn rari_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<RariBinary> {
        let mut args: Option<Vec<String>> = None;

        let (platform, arch) = zed::current_platform();
        let environment = match platform {
            zed::Os::Mac | zed::Os::Linux => Some(worktree.shell_env()),
            zed::Os::Windows => None,
        };

        if let Ok(lsp_settings) = LspSettings::for_worktree("mdn-lsp", worktree) {
            if let Some(binary) = lsp_settings.binary {
                args = binary.arguments;
                if let Some(path) = binary.path {
                    return Ok(RariBinary {
                        path: path.clone(),
                        args,
                        environment,
                    });
                }
            }
        }

        if let Some(path) = worktree.which("rari") {
            return Ok(RariBinary {
                path,
                args,
                environment,
            });
        }

        if let Some(path) = &self.binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(RariBinary {
                    path: path.clone(),
                    args,
                    environment,
                });
            }
        }

        let release = zed::latest_github_release(
            "mdn/rari",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let assert_name = match (arch, platform) {
            (Architecture::Aarch64, Os::Mac) => "rari-aarch64-apple-darwin.tar.gz",
            (Architecture::Aarch64, Os::Linux) => "rari-aarch64-unknown-linux-musl.tar.gz",
            (Architecture::Aarch64, Os::Windows) => "rari-aarch64-pc-windows-msvc.zip",
            (Architecture::X86, _) => return Err("x86 not supported".to_string()),
            (Architecture::X8664, Os::Mac) => "rari-x86_64-apple-darwin.tar.gz",
            (Architecture::X8664, Os::Linux) => "rari-x86_64-pc-windows-msvc.zip",
            (Architecture::X8664, Os::Windows) => "rari-x86_64-unknown-linux-musl.tar.gz",
        };

        let download_url = release
            .assets
            .into_iter()
            .find(|asset| asset.name == assert_name)
            .ok_or(format!("unable to find {assert_name} in latest release"))?
            .download_url;

        let version_dir = format!("rari-{}", release.version);
        let binary_path = match platform {
            zed::Os::Mac | zed::Os::Linux => format!("{version_dir}/rari"),
            zed::Os::Windows => format!("{version_dir}/rari.exe"),
        };

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &download_url,
                &version_dir,
                match platform {
                    zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
                    zed::Os::Windows => zed::DownloadedFileType::Zip,
                },
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.binary_path = Some(binary_path.clone());
        Ok(RariBinary {
            path: binary_path,
            args,
            environment,
        })
    }
}

impl zed::Extension for MDN {
    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let rari_binary = self.rari_binary(language_server_id, worktree)?;

        Ok(Command {
            command: rari_binary.path,
            args: once("lsp".to_string())
                .chain(rari_binary.args.unwrap_or_default())
                .collect(),
            env: once((
                "CONTENT_ROOT".to_string(),
                format!("{}/files", worktree.root_path()),
            ))
            .chain(rari_binary.environment.unwrap_or_default())
            .collect(),
        })
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        MDN { binary_path: None }
    }
}

zed::register_extension!(MDN);
