// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Drag-and-drop CurseForge modpack import.
//!
//! Dropping a pack's `.zip` or extracted folder on the window opens the
//! confirmation dialog: it shows the pack's name, Minecraft version and loader,
//! lets the user name the instance, and resolves the manifest's CurseForge file
//! list in the background. Confirming creates the instance and installs its
//! game, loader, files and overrides in the background.
//!
//! TODO: show the install's progress in the UI. Until that exists the progress
//! is written to the log only.
//!
//! A failed file lookup keeps the dialog open with the error and a Retry button:
//! the drop already named the pack and the user may have typed an instance name,
//! so closing the prompt on a network failure would throw that away.
//!
//! The OS file drop only reaches Rust through `window::on_window_event`, so the
//! filter is registered here; the actual work stays on the tokio runtime.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use shared::notification::{Notice, NoticeKind, NoticeSink};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, GameState, ModpackImportState, Navigation};
use crate::support::runtime;
use crate::ui::services::report;

pub(crate) fn setup(ui: &App, config: Rc<RefCell<config::Config>>, notices: NoticeSink) {
    // One import at a time, from the drop until its install finishes. A drop on
    // the launch page is rejected separately: a launch or install is already
    // running there.
    let busy = Arc::new(AtomicBool::new(false));
    let pending: Arc<Mutex<Option<modpack::Plan>>> = Arc::new(Mutex::new(None));
    let inspection: Arc<Mutex<Option<modpack::Inspection>>> = Arc::new(Mutex::new(None));
    let resolving: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    // A resolve that finishes after its dialog was cancelled (or replaced by a
    // new drop) must not write its plan or change the new dialog's state.
    let generation = Arc::new(AtomicU64::new(0));

    {
        let pending = Arc::clone(&pending);
        let inspection = Arc::clone(&inspection);
        let resolving = Arc::clone(&resolving);
        let busy = Arc::clone(&busy);
        let generation = Arc::clone(&generation);
        let dialog = ui.as_weak();
        ui.global::<ModpackImportState>().on_cancel(move || {
            generation.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut handle) = resolving.lock()
                && let Some(handle) = handle.take()
            {
                handle.abort();
            }
            if let Ok(mut slot) = pending.lock() {
                *slot = None;
            }
            if let Ok(mut slot) = inspection.lock() {
                *slot = None;
            }
            busy.store(false, Ordering::SeqCst);
            if let Some(ui) = dialog.upgrade() {
                let state = ui.global::<ModpackImportState>();
                state.set_checking(false);
                state.set_failed(false);
                state.set_error_message("".into());
                state.set_visible(false);
            }
        });
    }

    {
        let pending = Arc::clone(&pending);
        let inspection = Arc::clone(&inspection);
        let busy = Arc::clone(&busy);
        let notices = notices.clone();
        let weak = ui.as_weak();
        ui.global::<ModpackImportState>().on_confirm(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The button is hidden while checking/failed, but a click from a
            // stale frame must not install a plan that is not there.
            if ui.global::<ModpackImportState>().get_checking()
                || ui.global::<ModpackImportState>().get_failed()
            {
                return;
            }
            let name = ui
                .global::<ModpackImportState>()
                .get_instance_name()
                .to_string();
            let Some(plan) = pending.lock().ok().and_then(|mut slot| slot.take()) else {
                let state = ui.global::<ModpackImportState>();
                state.set_checking(false);
                state.set_visible(false);
                busy.store(false, Ordering::SeqCst);
                return;
            };
            if let Ok(mut slot) = inspection.lock() {
                *slot = None;
            }
            let state = ui.global::<ModpackImportState>();
            state.set_checking(false);
            state.set_visible(false);

            let config = config.borrow().clone();
            let busy_task = Arc::clone(&busy);
            let notices = notices.clone();
            let weak = weak.clone();
            runtime::spawn(async move {
                match modpack::create_instance(&plan, &name).await {
                    Ok(id) => {
                        log::info!("modpack instance {id} created; starting the install");
                        let created_id = id.clone();
                        report::report(&weak, move |ui| {
                            ui.global::<GameState>()
                                .invoke_refresh_selecting(created_id.into());
                        });
                        let result =
                            crate::usecases::modpack::install(id.clone(), plan, config).await;
                        match result {
                            Ok(()) => {
                                log::info!("modpack install into {id} finished");
                                notices(Notice::new(NoticeKind::TaskFinished));
                            }
                            Err(error) => {
                                log::error!("modpack install into {id} failed: {error}");
                                notices(
                                    Notice::new(NoticeKind::TaskFailed).with_arg(error.to_string()),
                                );
                            }
                        }
                        report::report(&weak, |ui| {
                            ui.global::<GameState>().invoke_refresh();
                        });
                    }
                    Err(error) => {
                        log::error!("could not create the modpack instance: {error}");
                        notices(Notice::new(NoticeKind::TaskFailed).with_arg(error.to_string()));
                    }
                }
                busy_task.store(false, Ordering::SeqCst);
            });
        });
    }

    {
        let pending = Arc::clone(&pending);
        let inspection_slot = Arc::clone(&inspection);
        let resolving = Arc::clone(&resolving);
        let generation = Arc::clone(&generation);
        let busy = Arc::clone(&busy);
        let weak = ui.as_weak();
        ui.global::<ModpackImportState>().on_retry(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(inspection) = inspection_slot.lock().ok().and_then(|slot| slot.clone()) else {
                let state = ui.global::<ModpackImportState>();
                state.set_checking(false);
                state.set_failed(false);
                state.set_visible(false);
                busy.store(false, Ordering::SeqCst);
                return;
            };
            if let Ok(mut slot) = pending.lock() {
                *slot = None;
            }
            let state = ui.global::<ModpackImportState>();
            state.set_checking(true);
            state.set_failed(false);
            state.set_error_message("".into());
            state.set_files(ModelRc::from(Rc::new(VecModel::from(
                Vec::<SharedString>::new(),
            ))));
            start_resolve(
                weak.clone(),
                inspection,
                Arc::clone(&inspection_slot),
                Arc::clone(&pending),
                Arc::clone(&resolving),
                Arc::clone(&generation),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let notices = notices.clone();
        let busy = Arc::clone(&busy);
        let pending = Arc::clone(&pending);
        let inspection_slot = Arc::clone(&inspection);
        let resolving = Arc::clone(&resolving);
        let generation = Arc::clone(&generation);
        window::on_window_event(ui, move |event| {
            let winit::event::WindowEvent::DroppedFile(path) = event else {
                return;
            };
            let Some(ui) = weak.upgrade() else { return };
            if ui.global::<Navigation>().get_current_page() == "launch" {
                notices(
                    Notice::new(NoticeKind::TaskFailed)
                        .with_arg("another install is already running"),
                );
                return;
            }
            if busy.swap(true, Ordering::SeqCst) {
                notices(
                    Notice::new(NoticeKind::TaskFailed)
                        .with_arg("another install is already running"),
                );
                return;
            }

            // The manifest read is a few milliseconds; showing the prompt before
            // the network call is what lets the user name the instance while
            // CurseForge is being asked for the file list.
            let inspection = match modpack::inspect(path) {
                Ok(inspection) => inspection,
                Err(error) => {
                    log::error!("modpack import could not start: {error}");
                    notices(Notice::new(NoticeKind::TaskFailed).with_arg(error.to_string()));
                    busy.store(false, Ordering::SeqCst);
                    return;
                }
            };
            if let Ok(mut slot) = pending.lock() {
                *slot = None;
            }
            if let Ok(mut slot) = inspection_slot.lock() {
                *slot = Some(inspection.clone());
            }

            let state = ui.global::<ModpackImportState>();
            state.set_pack_name(inspection.name().into());
            state.set_minecraft_version(inspection.minecraft().into());
            state.set_has_loader(inspection.loader_type().is_some());
            state.set_loader_type(
                inspection
                    .loader_type()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
                    .into(),
            );
            state.set_loader_version(inspection.loader_version().unwrap_or_default().into());
            state.set_instance_name(inspection.name().into());
            state.set_files(ModelRc::from(Rc::new(VecModel::from(
                Vec::<SharedString>::new(),
            ))));
            state.set_checking(true);
            state.set_failed(false);
            state.set_error_message("".into());
            state.set_visible(true);

            start_resolve(
                weak.clone(),
                inspection,
                Arc::clone(&inspection_slot),
                Arc::clone(&pending),
                Arc::clone(&resolving),
                Arc::clone(&generation),
            );
        });
    }
}

/// Resolves a pack and writes the outcome into the open dialog.
///
/// Success stores the plan and enables Install; failure keeps the dialog open
/// with the error and the inspection still in `inspection_slot`, so Retry can
/// run the same lookup again without re-reading the archive.
fn start_resolve(
    weak: slint::Weak<App>,
    inspection: modpack::Inspection,
    inspection_slot: Arc<Mutex<Option<modpack::Inspection>>>,
    pending: Arc<Mutex<Option<modpack::Plan>>>,
    resolving: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    generation: Arc<AtomicU64>,
) {
    let run = generation.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = runtime::spawn(async move {
        match modpack::resolve(inspection).await {
            Ok(plan) => {
                if generation.load(Ordering::SeqCst) != run {
                    return;
                }
                let files: Vec<SharedString> = plan
                    .unavailable()
                    .iter()
                    .map(|file| {
                        let scope = if file.required() {
                            "required"
                        } else {
                            "optional"
                        };
                        format!(
                            "{} — project {}, file {} ({scope}): {}",
                            file.file_name(),
                            file.project_id(),
                            file.file_id(),
                            file.reason()
                        )
                        .into()
                    })
                    .collect();
                if let Ok(mut slot) = pending.lock() {
                    *slot = Some(plan);
                }
                if let Ok(mut slot) = inspection_slot.lock() {
                    *slot = None;
                }
                report::report(&weak, move |ui| {
                    let state = ui.global::<ModpackImportState>();
                    state.set_failed(false);
                    state.set_error_message("".into());
                    state.set_files(ModelRc::from(Rc::new(VecModel::from(files))));
                    state.set_checking(false);
                });
            }
            Err(error) => {
                if generation.load(Ordering::SeqCst) != run {
                    return;
                }
                log::error!("modpack file resolution failed: {error}");
                report::report(&weak, move |ui| {
                    let state = ui.global::<ModpackImportState>();
                    state.set_checking(false);
                    state.set_failed(true);
                    state.set_error_message(error.to_string().into());
                });
            }
        }
    });
    if let Ok(mut slot) = resolving.lock() {
        if let Some(previous) = slot.take() {
            previous.abort();
        }
        *slot = Some(handle);
    }
}
