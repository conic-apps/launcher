// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral notification requests: plain data plus an opaque one-shot command,
//! so a domain crate can post a notice without depending on the interface.

use crate::Sink;

/// The category of a notice. The frontend owns the translated body for every
/// known kind; `with_arg` fills its placeholders in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// `args[0]` is the version that was staged.
    UpdateReady,
    /// `args[0]` is the version now running after an update was applied.
    UpdateApplied,
    TaskFinished,
    /// `args[0]` is the failure detail.
    TaskFailed,
    TaskCancelled,
    /// `args[0]` is the crashed instance's display name.
    GameCrashed,
    SetupSkipped,
    /// `args[0]` is the newly published Minecraft version.
    MinecraftReleased,
    /// The caller supplies the body; the frontend adds no copy.
    Custom,
}

/// A command the frontend runs when the user activates the notice. It may be
/// dropped unrun; a UI action captures a weak component handle itself because
/// this crate cannot name Slint types.
pub type NoticeAction = Box<dyn FnOnce() + Send + 'static>;

/// The output port a frontend implements to receive [`Notice`]s.
pub type NoticeSink = Sink<Notice>;

/// A notification request.
///
/// For a known [`NoticeKind`] the frontend owns the translated text and fills
/// its placeholders from [`Self::args`]. `title` and `body` override it with
/// literal, untranslated text.
pub struct Notice {
    pub kind: NoticeKind,
    /// Literal title above the body; overrides the kind's translated title.
    pub title: Option<String>,
    /// Literal body; overrides the kind's translated template.
    pub body: Option<String>,
    /// Values for the kind's template placeholders, in order.
    pub args: Vec<String>,
    /// A frontend icon name; leave unset to keep the kind's icon.
    pub icon: Option<String>,
    pub action: Option<NoticeAction>,
    /// Whether the frontend removes the card after running the action. When
    /// false the card stays visible but stops being clickable.
    pub dismiss_on_click: bool,
}

impl Notice {
    pub fn new(kind: NoticeKind) -> Self {
        Self {
            kind,
            title: None,
            body: None,
            args: Vec::new(),
            icon: None,
            action: None,
            dismiss_on_click: true,
        }
    }

    pub fn custom(body: impl Into<String>) -> Self {
        Self {
            body: Some(body.into()),
            ..Self::new(NoticeKind::Custom)
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_click(mut self, action: impl FnOnce() + Send + 'static) -> Self {
        self.action = Some(Box::new(action));
        self
    }

    pub fn dismiss_on_click(mut self, dismiss: bool) -> Self {
        self.dismiss_on_click = dismiss;
        self
    }

    pub fn keep_open_on_click(self) -> Self {
        self.dismiss_on_click(false)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    fn assert_send<T: Send>() {}

    #[test]
    fn a_notice_can_cross_a_thread_boundary() {
        assert_send::<Notice>();
    }

    #[test]
    fn the_builder_keeps_the_parts_together() {
        let hits = Arc::new(AtomicUsize::new(0));
        let hit = Arc::clone(&hits);
        let notice = Notice::new(NoticeKind::TaskFailed)
            .with_title("Network error")
            .with_arg("network")
            .with_icon("warning")
            .on_click(move || {
                hit.fetch_add(1, Ordering::SeqCst);
            });

        assert_eq!(notice.kind, NoticeKind::TaskFailed);
        assert_eq!(notice.title.as_deref(), Some("Network error"));
        assert_eq!(notice.args, ["network"]);
        assert!(notice.body.is_none());
        assert_eq!(notice.icon.as_deref(), Some("warning"));
        assert!(notice.dismiss_on_click);

        (notice.action.expect("an action"))();
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn custom_sets_the_kind_and_body() {
        let notice = Notice::custom("body");
        assert_eq!(notice.kind, NoticeKind::Custom);
        assert_eq!(notice.body.as_deref(), Some("body"));
        assert!(notice.title.is_none());
        assert!(notice.args.is_empty());
        assert!(notice.dismiss_on_click);
    }

    #[test]
    fn keeping_a_card_after_its_action_is_opt_in() {
        let notice = Notice::new(NoticeKind::TaskFinished).keep_open_on_click();
        assert!(!notice.dismiss_on_click);

        let notice = Notice::new(NoticeKind::TaskFinished).dismiss_on_click(false);
        assert!(!notice.dismiss_on_click);
    }
}
