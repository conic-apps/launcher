// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The page the browser lands on when the OAuth redirect fires.
//!
//! The page itself is [`page.html`], at the crate root: it is markup, it wants
//! to be read and previewed as markup, and everything in it — the rules, the
//! three glyphs, the layout — is the design, not logic. This module is the part
//! that cannot live in a stylesheet: which theme the launcher is in, and what
//! the two sentences say.
//!
//! The colours and the words are handed in, not looked up here. The launcher
//! reads them off its own `Theme` global and its own `@tr` catalog, so the page
//! is the app's live theme and the app's live language rather than a copy of
//! either that has to be kept in step.

use std::sync::LazyLock;

use log::error;
use regex::Regex;

/// The page, as written next door.
const TEMPLATE: &str = include_str!("../page.html");

/// One `{{ name }}` placeholder, however it is spaced.
///
/// The whitespace is optional on purpose. A value is as likely to land inside a
/// long attribute as on a line of its own — `--font-family` is the one that
/// does here — and the template should not have to care which, so
/// `{{ name }}`, `{{name}}` and `{{\n  name\n}}` are all the same placeholder.
///
/// The class is spelled out rather than written `\s`, which would be the
/// Unicode-aware Perl class: the only whitespace this ever has to match is
/// ASCII whitespace in a file in this crate, and the Unicode tables behind
/// `\s` are binary weight for nothing.
const PLACEHOLDER: &str = r"\{\{[ \t\r\n]*([A-Za-z][A-Za-z0-9-]*)[ \t\r\n]*\}\}";

/// The template's own documentation mentions the syntax, so it is matched
/// wherever it is rather than only in the places a value goes.
static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PLACEHOLDER).expect("the placeholder pattern is a literal"));

/// A colour for the page, as the app's `slint::Color` reduced to what CSS
/// takes.
///
/// Alpha is kept rather than composited here: the app's text tokens are
/// `default-text-color.transparentize(0.1)`, and handing the browser the same
/// 10% alpha over the same card colour lands on the same pixel the window does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 0.0 to 1.0, as `slint::Color::transparentize` leaves it.
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// The CSS colour this is, with the alpha at two decimals — enough for a
    /// browser and short enough to read in a stylesheet. A fully opaque colour
    /// drops the alpha rather than writing `, 1`.
    fn css(self) -> String {
        if self.a >= 1.0 {
            return self.opaque_css();
        }
        let alpha = (self.a.clamp(0.0, 1.0) * 100.0).round() / 100.0;
        format!("rgba({}, {}, {}, {alpha})", self.r, self.g, self.b)
    }

    /// The same colour, opaque — for the canvas behind the card, where there is
    /// nothing behind it to blend with.
    fn opaque_css(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

/// The theme the page is drawn in, read off the app's own tokens.
#[derive(Clone, Debug)]
pub struct Palette {
    /// `--window-background`: what the launcher window itself sits on.
    pub window: Rgba,
    /// `--dialog-background`: the card the page is drawn on.
    pub card: Rgba,
    /// `--dialog-border`.
    pub card_border: Rgba,
    /// The title, at the app-wide 0.9.
    pub title: Rgba,
    /// The sentence under it, one step quieter.
    pub body: Rgba,
    /// `--ctp-green`, on the success page.
    pub success: Rgba,
    /// `--ctp-red`, on the failure page.
    pub danger: Rgba,
    /// Whether the palette is one of the three dark flavors. `main.css` sets
    /// `color-scheme: dark` on `body`, and passing it on is what stops a Latte
    /// user from getting a white flash before the stylesheet is parsed.
    pub dark: bool,
    /// `--font-family-base`, the stack `main.css` sets on `body`.
    pub font_family: String,
}

/// The page's text, in the user's language.
///
/// The launcher supplies it from the `@tr` catalog rather than from a table
/// here, so a language is a catalog change like every other sentence in the app.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Shown on `/` — the page a browser reaches without the OAuth parameters.
    pub waiting_title: String,
    pub waiting_body: String,
    /// Shown once the code has been taken.
    pub success_title: String,
    pub success_body: String,
    /// Shown when the sign-in was refused, or the callback was not ours.
    pub failure_title: String,
    pub failure_body: String,
    /// The BCP 47 tag for the document's `lang`, from the resolved locale.
    pub language_tag: String,
}

/// Which of the three pages to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Waiting,
    Success,
    Failure,
}

impl Screen {
    /// The name the page's `data-screen` selects its glyph with.
    fn name(self) -> &'static str {
        match self {
            Screen::Waiting => "waiting",
            Screen::Success => "success",
            Screen::Failure => "failure",
        }
    }
}

/// Renders one of the three pages.
///
/// Filled in on the spot rather than at bind time: a request can be minutes
/// after the listener was bound, and this is one pass over a 4 KB template.
pub fn render(screen: Screen, palette: &Palette, messages: &Messages) -> String {
    let (title, body) = match screen {
        Screen::Waiting => (&messages.waiting_title, &messages.waiting_body),
        Screen::Success => (&messages.success_title, &messages.success_body),
        Screen::Failure => (&messages.failure_title, &messages.failure_body),
    };
    fill(screen, palette, title, body, messages)
}

/// Everything the template asks for, in one table.
///
/// The names are the template's, not this module's: a string that reaches the
/// page has to have a name the page can ask for by. That is the whole of the
/// i18n contract here — the launcher fills these from its `@tr` catalog, and
/// the page itself carries no sentence of its own.
fn fields(
    screen: Screen,
    palette: &Palette,
    title: &str,
    body: &str,
    messages: &Messages,
) -> Vec<(String, String)> {
    vec![
        // ----- the theme -----
        ("window".to_string(), palette.window.opaque_css()),
        ("card".to_string(), palette.card.opaque_css()),
        ("card-border".to_string(), palette.card_border.css()),
        ("title-color".to_string(), palette.title.css()),
        ("body-color".to_string(), palette.body.css()),
        ("success".to_string(), palette.success.css()),
        ("danger".to_string(), palette.danger.css()),
        (
            "color-scheme".to_string(),
            if palette.dark { "dark" } else { "light" }.to_string(),
        ),
        ("font-family".to_string(), palette.font_family.clone()),
        // ----- the two sentences, and which of the three screens they are -----
        ("screen".to_string(), screen.name().to_string()),
        ("lang".to_string(), messages.language_tag.clone()),
        ("title".to_string(), escape(title)),
        ("body".to_string(), escape(body)),
    ]
}

/// Substitutes the template's placeholders.
///
/// One pass over the matches rather than a `replace` per field, and both
/// directions are checked: a placeholder in the page with no field behind it
/// would reach the browser as literal `{{ … }}`, and a field nothing asks for
/// is a sentence or a token that silently stopped being shown. Both are
/// mistakes in *this* pair of files rather than anything a user can cause, and
/// the release profile aborts on a panic, so neither is allowed to stop the
/// app — they are logged and the page is served with the placeholder left as
/// it is, which is visible in the tab and is what a screenshot would show.
fn fill(screen: Screen, palette: &Palette, title: &str, body: &str, messages: &Messages) -> String {
    let fields = fields(screen, palette, title, body, messages);

    let page = PLACEHOLDER_RE
        .replace_all(TEMPLATE, |captures: &regex::Captures<'_>| {
            let name = captures.get(1).expect("the pattern has one group").as_str();
            match fields.iter().find(|(field, _)| field == name) {
                Some((_, value)) => value.clone(),
                None => {
                    error!("the callback page asks for '{{{{{name}}}}}' and nothing provides it");
                    captures
                        .get(0)
                        .expect("the match is there")
                        .as_str()
                        .to_string()
                }
            }
        })
        .into_owned();

    for (name, _) in &fields {
        if !asked_for(name) {
            error!("nothing on the callback page asks for '{name}'");
        }
    }
    page
}

/// Whether the template has a placeholder by this name.
fn asked_for(name: &str) -> bool {
    PLACEHOLDER_RE.is_match(TEMPLATE)
        && PLACEHOLDER_RE
            .captures_iter(TEMPLATE)
            .any(|captures| captures.get(1).is_some_and(|group| group.as_str() == name))
}

/// Escapes the five characters that can end a text node or an attribute early.
///
/// The sentences come from the app's own catalog, so this is correctness and
/// not a defence against anything: an ampersand or a quote in a translation has
/// to render as itself, and a `{{ title }}` carrying one is otherwise a broken
/// document.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            character => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette {
            window: Rgba::new(0x11, 0x11, 0x1b, 1.0),
            card: Rgba::new(0x1e, 0x1e, 0x2e, 1.0),
            card_border: Rgba::new(0x38, 0x3b, 0x41, 1.0),
            title: Rgba::new(0xcd, 0xd6, 0xf4, 0.9),
            body: Rgba::new(0xcd, 0xd6, 0xf4, 0.7),
            success: Rgba::new(0xa6, 0xe3, 0xa1, 1.0),
            danger: Rgba::new(0xf3, 0x8b, 0xa8, 1.0),
            dark: true,
            font_family: "'Comfortaa', system-ui, sans-serif".to_string(),
        }
    }

    fn messages() -> Messages {
        Messages {
            // Distinct from one another and from anything in `page.html`
            // itself — the header comment names the launcher, so a title of
            // "Conic Launcher" would be found on every page.
            waiting_title: "Waiting for the sign-in".to_string(),
            waiting_body: "Finish signing in <in the browser>".to_string(),
            success_title: "Signed in".to_string(),
            success_body: "You can close this tab.".to_string(),
            failure_title: "Sign-in failed".to_string(),
            failure_body: "Conic Launcher did not get a valid sign-in.".to_string(),
            language_tag: "en-US".to_string(),
        }
    }

    #[test]
    fn the_template_and_the_fields_agree_with_each_other() {
        // Both directions of the substitution, which is what the regex pass is
        // for: every placeholder the page has a value for, and every value
        // something on the page asks for. A key that drifts apart from the
        // template in either direction fails here rather than in a browser.
        for screen in [Screen::Waiting, Screen::Success, Screen::Failure] {
            let palette = palette();
            let messages = messages();
            let (title, body) = match screen {
                Screen::Waiting => (&messages.waiting_title, &messages.waiting_body),
                Screen::Success => (&messages.success_title, &messages.success_body),
                Screen::Failure => (&messages.failure_title, &messages.failure_body),
            };
            for (name, _) in fields(screen, &palette, title, body, &messages) {
                assert!(
                    asked_for(&name),
                    "'{name}' is provided but the page never asks for it ({screen:?})"
                );
            }
            for captures in PLACEHOLDER_RE.captures_iter(TEMPLATE) {
                let name = captures.get(1).expect("the pattern has one group").as_str();
                let provided = fields(screen, &palette, title, body, &messages)
                    .into_iter()
                    .any(|(field, _)| field == name);
                assert!(
                    provided,
                    "the page asks for '{name}' and nothing provides it"
                );
            }
        }
    }

    #[test]
    fn a_placeholder_is_the_same_however_it_is_spaced() {
        // The template is allowed to wrap one: `--font-family` lands inside a
        // long attribute, and reformatting the file should not break the page.
        let spaced = "{{\n   font-family\n}}";
        let tight = "{{font-family}}";
        let loose = "{{  font-family  }}";
        for form in [spaced, tight, loose] {
            let matched = PLACEHOLDER_RE
                .captures(form)
                .expect("a placeholder matches")
                .get(1)
                .expect("the pattern has a group")
                .as_str();
            assert_eq!(matched, "font-family", "in {form:?}");
        }
    }

    #[test]
    fn a_page_is_a_whole_document_with_nothing_to_fetch() {
        let page = render(Screen::Success, &palette(), &messages());
        assert!(page.starts_with("<!doctype html>"), "{}", &page[..40]);
        assert!(page.contains("<style>"));
        // Everything a browser would have to go and get is what must not be
        // there: the listener serves this one response and closes.
        assert!(!page.contains("http://"));
        assert!(!page.contains("https://"));
        assert!(!page.contains("url("));
        assert!(!page.contains("<script"));
    }

    #[test]
    fn the_page_carries_the_theme_it_was_given() {
        let page = render(Screen::Success, &palette(), &messages());
        assert!(page.contains("--card: rgb(30, 30, 46)"), "{page}");
        assert!(
            page.contains("--title-color: rgba(205, 214, 244, 0.9)"),
            "{page}"
        );
        assert!(page.contains("color-scheme: dark"), "{page}");
    }

    #[test]
    fn a_light_palette_asks_the_browser_for_a_light_form() {
        let mut light = palette();
        light.dark = false;
        assert!(render(Screen::Waiting, &light, &messages()).contains("color-scheme: light"));
    }

    #[test]
    fn each_screen_selects_its_own_glyph_and_accent() {
        let waiting = render(Screen::Waiting, &palette(), &messages());
        let success = render(Screen::Success, &palette(), &messages());
        let failure = render(Screen::Failure, &palette(), &messages());
        for (screen, page) in [
            (Screen::Waiting, &waiting),
            (Screen::Success, &success),
            (Screen::Failure, &failure),
        ] {
            assert!(
                page.contains(&format!(r#"data-screen="{}""#, screen.name())),
                "{page}"
            );
        }
        assert!(success.contains("M416 128 192 384l-96-96"));
        assert!(failure.contains("M85.57 446.25h340.86"));
        assert!(waiting.contains("M24 4A20 20 0 1 1 10.31 9.42"));
    }

    #[test]
    fn text_is_escaped_into_the_element_body() {
        let page = render(Screen::Waiting, &palette(), &messages());
        assert!(
            page.contains("Finish signing in &lt;in the browser&gt;"),
            "{page}"
        );
    }

    #[test]
    fn a_title_with_a_markup_character_in_it_does_not_break_the_head() {
        let mut altered = messages();
        altered.success_title = "Signed \"in\" & done".to_string();
        let page = render(Screen::Success, &palette(), &altered);
        assert!(
            page.contains("<title>Signed &quot;in&quot; &amp; done</title>"),
            "{page}"
        );
    }

    #[test]
    fn a_sentence_that_looks_like_a_placeholder_is_not_one() {
        // The pass walks the template, not its own output, so a translated
        // sentence containing the syntax survives it — which matters because a
        // translator writes whatever they like and some languages bracket
        // heavily.
        let mut awkward = messages();
        awkward.success_body = "{{ title }} and {{ body }} are literal here".to_string();
        let page = render(Screen::Success, &palette(), &awkward);
        assert!(
            page.contains("{{ title }} and {{ body }} are literal here"),
            "{page}"
        );
    }

    #[test]
    fn the_three_screens_take_their_own_sentences() {
        // The i18n end of the contract: a page shows the pair it was asked for
        // and neither of the other two. (Only the titles are compared, because
        // a body goes through `escape` and a fixture body with markup in it
        // would not be found verbatim — `text_is_escaped_into_the_element_body`
        // is what covers that.)
        let pairs = [
            (
                Screen::Waiting,
                "Waiting for the sign-in",
                "Signed in",
                "Sign-in failed",
            ),
            (
                Screen::Success,
                "Signed in",
                "Waiting for the sign-in",
                "Sign-in failed",
            ),
            (
                Screen::Failure,
                "Sign-in failed",
                "Waiting for the sign-in",
                "Signed in",
            ),
        ];
        for (screen, own, first_other, second_other) in pairs {
            let page = render(screen, &palette(), &messages());
            assert!(page.contains(own), "{screen:?} lost its own title:\n{page}");
            assert!(
                !page.contains(first_other),
                "{screen:?} shows a foreign title"
            );
            assert!(
                !page.contains(second_other),
                "{screen:?} shows a foreign title"
            );
        }
    }

    #[test]
    fn the_language_tag_reaches_the_document() {
        let mut translated = messages();
        translated.language_tag = "zh-CN".to_string();
        assert!(render(Screen::Failure, &palette(), &translated).contains(r#"lang="zh-CN""#));
    }
}
