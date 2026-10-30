// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! UI-neutral notification requests.
//!
//! A notice crosses the boundary as plain data plus an opaque one-shot command:
//! it mentions neither Slint nor the app, so a domain crate can post one without
//! depending on the interface. The app implements the [`NoticeSink`] port and
//! owns the action's lifetime.

use crate::Sink;

/// The category of a notice.
///
/// The kind selects the frontend's translated body; [`Self::Custom`] leaves it
/// to the caller. A fixed message belongs here, in the frontend's catalogue,
/// rather than as a literal at the call site.
///
/// A new translated template is a new variant here plus a matching branch in
/// the frontend's `NotificationText`; [`Notice::with_arg`] fills its `{}`
/// placeholders in order. A body like "The {name} instance finished" therefore
/// becomes `@tr("The {} instance finished", arg(item, 0))` in the frontend and
/// `Notice::new(kind).with_arg(name)` at the call site. A crate must not format
/// UI copy itself: it has no catalogue, so the frontend owns every fixed
/// sentence and the crate only supplies the values that vary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// `args[0]` is the version that was staged.
    UpdateReady,
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

/// A command the frontend runs when the user activates the notice.
///
/// The action is a one-shot value: it may be dropped unrun when the notice is
/// dismissed or cleared. It deliberately takes no `&App` — [`crate`] must not
/// name Slint types, so a UI action captures a weak component handle itself.
pub type NoticeAction = Box<dyn FnOnce() + Send + 'static>;

/// The output port a frontend implements to receive [`Notice`]s.
///
/// Implementations must accept calls from any thread; the app's implementation
/// marshals onto its event loop. This is the same shape as the other `Sink`
/// ports in this crate.
pub type NoticeSink = Sink<Notice>;

/// A notification request.
///
/// For a known [`NoticeKind`] the frontend owns the translated template and
/// fills its placeholders from [`Self::args`]. [`Self::body`] overrides that
/// template with literal text; a [`NoticeKind::Custom`] notice sets it because
/// it has no default copy. Literal text is shown as-is and is not translated.
pub struct Notice {
    pub kind: NoticeKind,
    /// Literal title shown above the body, or `None` for a body-only notice.
    ///
    /// A known kind may already have a translated title in the frontend; this
    /// overrides it. Like [`Self::body`], the text is shown as-is and is not
    /// translated, so a crate with localizable copy should leave it unset and
    /// let the frontend own the title.
    pub title: Option<String>,
    /// Literal body, or `None` for the kind's translated template.
    pub body: Option<String>,
    /// Values for the kind's template placeholders, in order.
    pub args: Vec<String>,
    /// A frontend icon name, or `None` for the kind's icon. Crates should leave
    /// this alone so they do not depend on UI assets.
    pub icon: Option<String>,
    /// Runs once on the frontend thread when the card is activated.
    pub action: Option<NoticeAction>,
    /// Whether the frontend removes the card after running the action.
    ///
    /// Defaults to true. When false the card stays in the centre but stops
    /// being clickable, because its one-shot action has been spent.
    pub dismiss_on_click: bool,
}

impl Notice {
    /// A notice whose body the frontend takes from `kind`.
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

    /// A notice with a caller-supplied, untranslated body.
    pub fn custom(body: impl Into<String>) -> Self {
        Self {
            body: Some(body.into()),
            ..Self::new(NoticeKind::Custom)
        }
    }

    /// Appends one template argument.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds a literal title above the body.
    ///
    /// Optional: without it the card keeps the single-body layout. The text is
    /// not translated; leave it unset for localizable copy.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replaces the kind's body with literal text.
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Replaces the kind's icon with a frontend icon name.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Attaches the action run when the user activates the card.
    pub fn on_click(mut self, action: impl FnOnce() + Send + 'static) -> Self {
        self.action = Some(Box::new(action));
        self
    }

    /// Chooses whether the card is removed after its action runs.
    pub fn dismiss_on_click(mut self, dismiss: bool) -> Self {
        self.dismiss_on_click = dismiss;
        self
    }

    /// Keeps the card visible after its action runs; it stops being clickable.
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
