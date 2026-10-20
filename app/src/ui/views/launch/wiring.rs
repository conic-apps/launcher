// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch page's callbacks, registered on `LaunchState`.

use super::*;

pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    SHARED_CONFIG.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&config)));
    let controller = Rc::new(RefCell::new(LaunchController::new()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));
    let state = ui.global::<LaunchState>();

    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_start(move || {
            let Some(ui) = weak.upgrade() else { return };
            let current_id = ui.global::<GameState>().get_current_id().to_string();
            let Some(token) = controller.borrow_mut().begin(current_id.clone()) else {
                return;
            };
            crate::ui::services::notifications::start_task(&ui, "launch");

            // Everything the flow needs that is not on disk is gathered on the
            // UI thread, because `GameState`/`LaunchState` and the config are
            // not reachable from the runtime.
            let config_snapshot = SHARED_CONFIG.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|config| config.borrow().clone())
                    .unwrap_or_default()
            });

            // `instance.toml` is read on the runtime, and the state is reset
            // behind that read.
            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                let instance = instance::get_instance_by_id(&current_id).await;
                let loader = instance
                    .as_ref()
                    .and_then(|instance| instance.config.runtime.mod_loader_type.as_ref())
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default();
                let flow_weak = weak.clone();
                let flow_config = config_snapshot.clone();
                crate::ui::services::report::report(&weak, move |ui| {
                    reset_state(&ui, instance.as_ref(), &flow_config);
                    let sink = update_sink(flow_weak.clone(), token, loader);
                    let task = crate::support::runtime::spawn(async move {
                        crate::usecases::launch::run(flow_config, instance, sink).await;
                    });
                    launch_controller().borrow_mut().task = Some(task);
                });
            });
        });
    }

    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_cancel(move || {
            let had_task = controller.borrow_mut().cancel();
            if let Some(ui) = weak.upgrade() {
                crate::ui::services::notifications::cancel_task(&ui);
                ui.global::<Navigation>().invoke_back();
                if had_task {
                    ui.global::<GameState>().invoke_refresh();
                }
            }
        });
    }

    {
        // The launch task deliberately outlives the launch page. Navigation
        // only changes where progress is displayed; cancellation is explicit
        // through the launch page's cancel action.
    }
}
