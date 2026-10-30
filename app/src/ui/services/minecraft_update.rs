// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The notification for a newly published Minecraft version.
//!
//! Like the self-update check, this runs once at startup in the background. The
//! comparison and the record of what was already shown live in
//! [`crate::usecases::minecraft_update`]; this module only turns a published
//! version into a notice whose action opens that version's wiki page.

use std::{cell::RefCell, rc::Rc};

use shared::notification::{Notice, NoticeKind, NoticeSink};

use crate::support::runtime;
use crate::ui::services::app_config;
use crate::usecases::minecraft_update;

pub(crate) fn setup(shared: Rc<RefCell<config::Config>>, notices: NoticeSink) {
    let (release_enabled, snapshot_enabled, language) = {
        let config = shared.borrow();
        (
            config.accessibility.release_reminder,
            config.accessibility.snapshot_reminder,
            app_config::active_language_code(config.language.as_deref()),
        )
    };

    runtime::spawn(async move {
        for version in minecraft_update::check(release_enabled, snapshot_enabled).await {
            let language = language.clone();
            let notice_version = version.clone();
            notices(
                Notice::new(NoticeKind::MinecraftReleased)
                    .with_arg(version)
                    .on_click(move || {
                        app_config::open_minecraft_version_wiki(&language, &notice_version);
                    }),
            );
        }
    });
}
