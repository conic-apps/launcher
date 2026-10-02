// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch page's callbacks, registered on `LaunchState`.

use super::*;

/// Registers the launch view's callbacks.
pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    SHARED_CONFIG.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&config)));
    let controller = Rc::new(RefCell::new(LaunchController::new()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));
    let state = ui.global::<LaunchState>();

    // The launch button: cancel any running flow and start a new one.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_start(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().cancel();
            let token = controller.borrow_mut().begin();

            // Everything the flow needs that is not on disk is gathered on the
            // UI thread, because `GameState`/`LaunchState` and the config are
            // not reachable from the runtime.
            let current_id = ui.global::<GameState>().get_current_id().to_string();
            let config_snapshot = SHARED_CONFIG.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|config| config.borrow().clone())
                    .unwrap_or_default()
            });

            // `instance.toml` is read on the runtime, and the state is reset
            // behind that read.
            let weak = weak.clone();
            crate::runtime::spawn(async move {
                let instance = instance::get_instance_by_id(&current_id).await;
                let flow_weak = weak.clone();
                let flow_config = config_snapshot.clone();
                crate::report::report(&weak, move |ui| {
                    reset_state(&ui, instance.as_ref(), &flow_config);
                    let task = crate::runtime::spawn(async move {
                        run_flow(flow_weak, token, config_snapshot, instance).await;
                    });
                    launch_controller().borrow_mut().task = Some(task);
                });
            });
        });
    }

    // The back button. Leaving the page cancels the flow too.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_cancel(move || {
            let had_task = controller.borrow_mut().cancel();
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_back();
                if had_task {
                    ui.global::<GameState>().invoke_refresh();
                }
            }
        });
    }

    // Leaving the page (title bar Home/Settings) cancels the flow.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<Navigation>().on_page_changed(move |page| {
            if page.as_str() != "launch" && controller.borrow_mut().cancel() {
                // The instance list is reloaded, so `.install.lock` written by
                // a fresh install shows up on the game screen.
                if let Some(ui) = weak.upgrade() {
                    ui.global::<GameState>().invoke_refresh();
                }
            }
        });
    }
}
