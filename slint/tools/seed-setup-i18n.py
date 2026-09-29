#!/usr/bin/env python3
"""Seed the setup wizard's strings into the twelve gettext catalogs.

The msgids are the `@tr()` source strings under `slint/app/ui/views/setup*` and
the msgstrs come from `setup.*` in `src/locales/*.ts` (the Vue's own catalogs,
which are the translations the app already ships). Written as a one-shot helper
rather than kept in the tree: the catalogs are the artefact, and the wizard's
strings are seeded from the Vue exactly once.

    python3 slint/tools/seed-setup-i18n.py
"""

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

LANGS = {
    "zh_cn": "zh_CN",
    "zh_tw": "zh_TW",
    "ja_jp": "ja_JP",
    "ko_kr": "ko_KR",
    "de_de": "de_DE",
    "fr_fr": "fr_FR",
    "es_es": "es_ES",
    "pt_br": "pt_BR",
    "ru_ru": "ru_RU",
    "tr_tr": "tr_TR",
    "pl_pl": "pl_PL",
    "en_us": "en_US",
}

# (msgctxt, msgid, the `setup.*` leaf in src/locales/*.ts it is seeded from).
# The wizard's six screens, in the order they appear in the .slint files.
ENTRIES = [
    ("SetupView", "First-run setup", "welcome"),
    ("SetupView", "Customize Conic Launcher to suit your preferences", "welcomeDesc"),
    ("SetupView", "Back", "back"),
    ("SetupView", "Let's get started!", "steps.start"),
    ("SetupView", "Next (Add account)", "steps.addAccount"),
    ("SetupView", "Next (Java settings)", "steps.javaSettings"),
    ("SetupView", "Next (Launch options)", "steps.launchOptions"),
    ("SetupView", "Next (Import instances)", "steps.importInstances"),
    ("SetupView", "Finish", "steps.finish"),

    ("SetupLanguage", "Welcome", "language.title"),
    ("SetupLanguage", "Welcome to the first-run setup wizard!", "language.desc"),
    (
        "SetupLanguage",
        "Conic Launcher is highly customizable, but the complex settings might be overwhelming.",
        "language.desc2",
    ),
    (
        "SetupLanguage",
        "This wizard will quickly walk you through some important settings so you can start playing right away.",
        "language.desc3",
    ),

    ("SetupPalette", "Choose a color scheme", "palette.title"),
    (
        "SetupPalette",
        "Conic Launcher supports multiple color schemes. Pick one that suits your style.",
        "palette.desc",
    ),
    ("SetupPalette", "Follow system dark mode", "palette.followSystem"),
    (
        "SetupPalette",
        "Uses Latte when the system is set to light mode, otherwise uses Mocha",
        "palette.followSystemDesc",
    ),

    ("SetupAddAccount", "Account added!", "addAccount.added"),
    (
        "SetupAddAccount",
        "The following profile has been set as default. The launcher will use this profile to log in to the game when no profile is assigned to an instance",
        "addAccount.profileDesc",
    ),
    ("SetupAddAccount", "Microsoft (Official account)", "addAccount.microsoft"),
    ("SetupAddAccount", "Yggdrasil (External login)", "addAccount.yggdrasil"),
    ("SetupAddAccount", "No auth service (Offline account)", "addAccount.offline"),

    ("SetupJavaSettings", "Java settings", "javaSettings.title"),
    (
        "SetupJavaSettings",
        'Conic Launcher can greatly simplify Java environment configuration. Enable "Auto-install Java runtime" to avoid manual installation!',
        "javaSettings.desc",
    ),
    (
        "SetupJavaSettings",
        'Note: Auto-installation of Java may not be available on the current platform. It is recommended to install Java manually and disable "Auto-install Java runtime"',
        "javaSettings.note",
    ),

    ("SetupLaunchOptions", "Game launch options", "launchOptions.title"),
    (
        "SetupLaunchOptions",
        "Here are some basic launch options. You can change more options later in Settings and instance-specific settings.",
        "launchOptions.desc",
    ),

    # `importInstances.title` and `importInstances.importButton` are the same
    # sentence, so one entry answers both call sites.
    ("SetupImportInstances", "Import instances", "importInstances.title"),
    (
        "SetupImportInstances",
        "Import previously played instances from other launchers to get started quickly",
        "importInstances.desc",
    ),
    (
        "SetupImportInstances",
        "If you're new to Minecraft, you can also create a blank instance with the latest version right away",
        "importInstances.newPlayerHint",
    ),
    ("SetupImportInstances", "Latest release", "importInstances.latestRelease"),
    ("SetupImportInstances", "Latest snapshot", "importInstances.latestSnapshot"),
    ("SetupImportInstances", "An error occurred", "importInstances.error"),
    ("SetupImportInstances", "Creating...", "importInstances.creating"),
    (
        "SetupImportInstances",
        "Create instance with latest release",
        "importInstances.createRelease",
    ),
    (
        "SetupImportInstances",
        "Create instance with latest snapshot",
        "importInstances.createSnapshot",
    ),
]


def read_setup_block(path):
    """The `setup: { … }` object of one `src/locales/*.ts`, as a flat dict."""
    with open(path, encoding="utf-8") as handle:
        source = handle.read()
    start = source.find("\n    setup: {")
    if start < 0:
        raise SystemExit(f"no `setup:` block in {path}")
    start = source.index("{", start)
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return flatten(source[start : index + 1])
    raise SystemExit(f"unterminated `setup:` block in {path}")


def flatten(block, prefix=""):
    out = {}
    index = 1
    while index < len(block) - 1:
        match = re.match(r"\s*([A-Za-z0-9_]+)\s*:\s*", block[index:])
        if not match:
            index += 1
            continue
        key = prefix + match.group(1)
        index += match.end()
        if block[index] == "{":
            depth = 0
            for end in range(index, len(block)):
                if block[end] == "{":
                    depth += 1
                elif block[end] == "}":
                    depth -= 1
                    if depth == 0:
                        break
            out.update(flatten(block[index : end + 1], key + "."))
            index = end + 1
            continue
        if block[index] in "\"'":
            quote = block[index]
            end = index + 1
            while True:
                if block[end] == "\\":
                    end += 2
                    continue
                if block[end] == quote:
                    break
                end += 1
            out[key] = block[index + 1 : end]
            index = end + 1
    return out


def po_quote(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    translations = {
        vue: read_setup_block(os.path.join(ROOT, "src", "locales", f"{vue}.ts"))
        for vue in LANGS
    }
    for vue, table in translations.items():
        for _, _, key in ENTRIES:
            if key not in table:
                raise SystemExit(f"{vue}.ts has no `setup.{key}`")

    for vue, slint in LANGS.items():
        path = os.path.join(
            ROOT, "slint", "app", "i18n", slint, "LC_MESSAGES", "conic-launcher-slint.po"
        )
        with open(path, encoding="utf-8") as handle:
            catalog = handle.read()
        if "msgctxt \"SetupView\"" in catalog:
            print(f"{slint}: already seeded, skipping")
            continue
        table = translations[vue]
        if not catalog.endswith("\n"):
            catalog += "\n"
        for context, msgid, key in ENTRIES:
            catalog += (
                "#: ui\n"
                f"msgctxt {po_quote(context)}\n"
                f"msgid {po_quote(msgid)}\n"
                f"msgstr {po_quote(table[key])}\n"
            )
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(catalog)
        print(f"{slint}: {len(ENTRIES)} entries")

    print(json.dumps({"entries": len(ENTRIES)}))


if __name__ == "__main__":
    main()
