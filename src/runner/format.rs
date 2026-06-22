// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use crate::backend::Backend;
use crate::catalog::linter::{DetectionResult, OwnedToolDef};
use crate::runner::{Invocation, LintOutput, Mode, run_linters};

#[must_use]
pub fn format(
    detection: &DetectionResult,
    chain: &[Box<dyn Backend>],
    tools: &[OwnedToolDef],
    workspace: &Path,
    mode: Mode,
    invocation: Invocation,
) -> LintOutput {
    run_linters(detection, chain, tools, workspace, mode, invocation)
}
