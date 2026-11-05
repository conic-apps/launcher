// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The page the browser lands on when the OAuth redirect fires. The colors and
//! words are handed in through `Palette` and `Messages`.

use std::sync::LazyLock;

use log::error;
use regex::Regex;

const TEMPLATE: &str = include_str!("page.html");

/// `page.html` carries the license header hawkeye requires, but the document
/// served to the browser has to start with its doctype, so the leading comment
/// is dropped here.
fn template() -> &'static str {
    TEMPLATE
        .strip_prefix("<!--")
        .and_then(|rest| rest.split_once("-->"))
        .map_or(TEMPLATE, |(_, rest)| rest.trim_start())
}

/// One `{{ name }}` placeholder, however it is spaced. The whitespace class is
/// spelled out rather than `\s` because only ASCII whitespace in this crate's
/// own file has to match.
const PLACEHOLDER: &str = r"\{\{[ \t\r\n]*([A-Za-z][A-Za-z0-9-]*)[ \t\r\n]*\}\}";

static PLACEHOLDER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PLACEHOLDER).expect("the placeholder pattern is a literal"));

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 0.0 to 1.0.
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }

    fn css(self) -> String {
        if self.a >= 1.0 {
            return self.opaque_css();
        }
        let alpha = (self.a.clamp(0.0, 1.0) * 100.0).round() / 100.0;
        format!("rgba({}, {}, {}, {alpha})", self.r, self.g, self.b)
    }

    fn opaque_css(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

/// The theme the page is drawn in, read off the app's own tokens.
#[derive(Clone, Debug)]
pub struct Palette {
    pub window: Rgba,
    pub card: Rgba,
    pub card_border: Rgba,
    pub title: Rgba,
    pub body: Rgba,
    pub success: Rgba,
    pub danger: Rgba,
    pub dark: bool,
    pub font_family: String,
}

/// The page's text, in the user's language.
#[derive(Clone, Debug)]
pub struct Messages {
    pub waiting_title: String,
    pub waiting_body: String,
    pub success_title: String,
    pub success_body: String,
    pub failure_title: String,
    pub failure_body: String,
    pub language_tag: String,
}

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
/// after the listener was bound.
pub fn render(screen: Screen, palette: &Palette, messages: &Messages) -> String {
    let (title, body) = match screen {
        Screen::Waiting => (&messages.waiting_title, &messages.waiting_body),
        Screen::Success => (&messages.success_title, &messages.success_body),
        Screen::Failure => (&messages.failure_title, &messages.failure_body),
    };
    fill_template_placeholder(screen, palette, title, body, messages)
}

fn template_fields(
    screen: Screen,
    palette: &Palette,
    title: &str,
    body: &str,
    messages: &Messages,
) -> Vec<(String, String)> {
    vec![
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
        ("screen".to_string(), screen.name().to_string()),
        ("lang".to_string(), messages.language_tag.clone()),
        ("title".to_string(), escape(title)),
        ("body".to_string(), escape(body)),
    ]
}

fn fill_template_placeholder(
    screen: Screen,
    palette: &Palette,
    title: &str,
    body: &str,
    messages: &Messages,
) -> String {
    let fields = template_fields(screen, palette, title, body, messages);

    let page = PLACEHOLDER_REGEX
        .replace_all(template(), |captures: &regex::Captures<'_>| {
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

fn asked_for(name: &str) -> bool {
    PLACEHOLDER_REGEX.is_match(template())
        && PLACEHOLDER_REGEX
            .captures_iter(template())
            .any(|captures| captures.get(1).is_some_and(|group| group.as_str() == name))
}

/// Escapes the five characters that can end a text node or an attribute early.
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
        for screen in [Screen::Waiting, Screen::Success, Screen::Failure] {
            let palette = palette();
            let messages = messages();
            let (title, body) = match screen {
                Screen::Waiting => (&messages.waiting_title, &messages.waiting_body),
                Screen::Success => (&messages.success_title, &messages.success_body),
                Screen::Failure => (&messages.failure_title, &messages.failure_body),
            };
            for (name, _) in template_fields(screen, &palette, title, body, &messages) {
                assert!(
                    asked_for(&name),
                    "'{name}' is provided but the page never asks for it ({screen:?})"
                );
            }
            for captures in PLACEHOLDER_REGEX.captures_iter(template()) {
                let name = captures.get(1).expect("the pattern has one group").as_str();
                let provided = template_fields(screen, &palette, title, body, &messages)
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
        let spaced = "{{\n   font-family\n}}";
        let tight = "{{font-family}}";
        let loose = "{{  font-family  }}";
        for form in [spaced, tight, loose] {
            let matched = PLACEHOLDER_REGEX
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
