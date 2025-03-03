use std::ffi::OsString;
use std::process::ExitStatus;

use anyhow::Result;

use crate::{
    cli::{common::set_globals, job, self_update},
    command::run_command_for_dir,
    config::Cfg,
    currentprocess::{argsource::ArgSource, process},
    toolchain::names::{LocalToolchainName, ResolvableLocalToolchainName},
    utils::utils,
};

#[cfg_attr(feature = "otel", tracing::instrument)]
pub async fn main(arg0: &str) -> Result<ExitStatus> {
    self_update::cleanup_self_updater()?;

    let _setup = job::setup();

    let mut args = process().args_os().skip(1);

    // Check for a + toolchain specifier
    let arg1 = args.next();
    let toolchain = arg1
        .as_ref()
        .map(|arg| arg.to_string_lossy())
        .filter(|arg| arg.starts_with('+'))
        .map(|name| ResolvableLocalToolchainName::try_from(&name.as_ref()[1..]))
        .transpose()?;

    // Build command args now while we know whether or not to skip arg 1.
    let cmd_args: Vec<_> = crate::currentprocess::process()
        .args_os()
        .skip(1 + toolchain.is_some() as usize)
        .collect();

    let cfg = set_globals(false, true)?;
    cfg.check_metadata_version()?;
    let toolchain = toolchain
        .map(|t| t.resolve(&cfg.get_default_host_triple()?))
        .transpose()?;
    direct_proxy(&cfg, arg0, toolchain, &cmd_args).await
}

#[cfg_attr(feature = "otel", tracing::instrument(skip(cfg)))]
async fn direct_proxy(
    cfg: &Cfg,
    arg0: &str,
    toolchain: Option<LocalToolchainName>,
    args: &[OsString],
) -> Result<ExitStatus> {
    let cmd = match toolchain {
        None => {
            cfg.create_command_for_dir(&utils::current_dir()?, arg0)
                .await?
        }
        Some(tc) => cfg.create_command_for_toolchain(&tc, false, arg0).await?,
    };
    run_command_for_dir(cmd, arg0, args)
}
