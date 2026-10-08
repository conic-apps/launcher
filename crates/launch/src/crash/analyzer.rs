// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Recognising why a game crashed.
//!
//! A port of HMCL's `CrashReportAnalyzer` (GPL-3.0), whose rule table is the
//! collected experience of several years of Minecraft crash reports. Each rule
//! is a regex over the game's output; the first match per rule becomes a
//! [`Reason`] that names the rule and carries the groups it captured (a mod id,
//! an expected Java version, a file, …).
//!
//! The module also finds the crash report itself the way HMCL does: from the
//! game's own `#@!@# Game crashed! Crash report saved to:` line (exact path),
//! or, failing that, extracted from the captured console output. This avoids
//! guessing by modified time, which can surface a report from an earlier run.
//!
//! Two Java-only constructs are spelled out for Rust's `regex`: `\R` (any line
//! break) becomes `\r?\n`, and the named-group syntax is the same
//! `(?<name>…)`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use log::warn;
use regex::Regex;

/// One recognised crash cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reason {
    /// The rule's name, e.g. `"OUT_OF_MEMORY"`. Stable enough to map to a
    /// message on the UI side.
    pub rule: &'static str,
    /// The captured named groups, in the rule's own order.
    pub fields: Vec<String>,
}

/// A compiled rule: its name, regex and the groups that carry its fields.
struct Rule {
    name: &'static str,
    regex: Regex,
    groups: &'static [&'static str],
}

/// The whole table, compiled once, on the first crash. A pattern that fails to
/// compile is dropped with a warning rather than taking the launcher down.
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    RULES_SOURCE
        .iter()
        .filter_map(|(name, pattern, groups)| match Regex::new(pattern) {
            Ok(regex) => Some(Rule {
                name,
                regex,
                groups,
            }),
            Err(error) => {
                warn!("crash rule {name} does not compile: {error}");
                None
            }
        })
        .collect()
});

/// Runs every rule over `log` and returns the matches, one per rule.
pub fn analyze(log: &str) -> Vec<Reason> {
    let mut reasons = Vec::new();
    for rule in RULES.iter() {
        let Some(captures) = rule.regex.captures(log) else {
            continue;
        };
        let fields = rule
            .groups
            .iter()
            .map(|name| {
                captures
                    .name(name)
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect();
        reasons.push(Reason {
            rule: rule.name,
            fields,
        });
    }
    reasons
}

/// The game's own crash-report location line, with the file's contents.
///
/// The line is `#@!@# Game crashed! Crash report saved to: #@!@# <path>`, so the
/// report is read exactly where the game wrote it, even under a custom game
/// directory.
pub fn find_crash_report(log: &str) -> Option<(PathBuf, String)> {
    static LOCATION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"#@!@# Game crashed! Crash report saved to: #@!@# (?<location>.*)")
            .expect("the crash-report location pattern is a literal")
    });
    let path = LOCATION.captures(log)?.name("location")?.as_str().trim();
    match fs::read_to_string(Path::new(path)) {
        Ok(contents) => Some((PathBuf::from(path), contents)),
        Err(error) => {
            warn!("could not read the crash report at {path}: {error}");
            None
        }
    }
}

/// The crash report carved out of the console output, for when the game wrote
/// no file we can reach.
pub fn extract_crash_report(log: &str) -> Option<String> {
    let begin = log.rfind("---- Minecraft Crash Report ----")?;
    let end = log.rfind("#@!@# Game crashed! Crash report saved to")?;
    (begin < end).then(|| log[begin..end].to_string())
}

/// Likely offending mods, inferred from a crash report's stack trace.
///
/// Each `at package.Class.method(…)` line contributes its package segments,
/// minus the ones every crash shares (Java, Minecraft, the loaders).
pub fn find_keywords(crash_report: &str) -> Vec<String> {
    static STACK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"Description: (.*?)[\n\r]+(?<stacktrace>[\w\W\n\r]+)A detailed walkthrough of the error")
            .expect("the crash-report stack pattern is a literal")
    });
    static LINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"at (?<method>.*?)\((?<sourcefile>.*?)\)")
            .expect("the stack-trace line pattern is a literal")
    });
    static MODULE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\{(?<tokens>.*)}").expect("the module-token pattern is a literal")
    });

    let mut result = BTreeSet::new();
    let Some(captures) = STACK.captures(crash_report) else {
        return Vec::new();
    };
    for line in captures["stacktrace"].split('\n') {
        if let Some(line) = LINE.captures(line) {
            for segment in line["method"].split('.') {
                let segment = segment.trim();
                if segment.is_empty() {
                    continue;
                }
                result.insert(segment.to_string());
            }
        }
        if let Some(module) = MODULE.captures(line) {
            for token in module["tokens"].split(',') {
                let parts: Vec<&str> = token.split(':').collect();
                if parts.len() >= 2 && parts[0] == "xf" && !parts[1].is_empty() {
                    result.insert(parts[1].to_string());
                }
            }
        }
    }
    result
        .into_iter()
        .filter(|keyword| !PACKAGE_KEYWORD_BLACK_LIST.contains(&keyword.as_str()))
        .collect()
}

/// Package segments that name Java, Minecraft or a loader rather than the mod
/// that might be at fault. Ported from HMCL, whitespace removed.
const PACKAGE_KEYWORD_BLACK_LIST: &[&str] = &[
    "net",
    "minecraft",
    "item",
    "setup",
    "block",
    "assist",
    "optifine",
    "player",
    "unimi",
    "fastutil",
    "tileentity",
    "events",
    "common",
    "blockentity",
    "client",
    "entity",
    "mojang",
    "main",
    "gui",
    "world",
    "server",
    "dedicated",
    "map",
    "dsi",
    "renderer",
    "chunk",
    "model",
    "loading",
    "color",
    "pipeline",
    "inventory",
    "launcher",
    "physics",
    "particle",
    "gen",
    "registry",
    "worldgen",
    "texture",
    "biomes",
    "biome",
    "monster",
    "passive",
    "ai",
    "integrated",
    "tile",
    "state",
    "play",
    "override",
    "transformers",
    "structure",
    "nbt",
    "pathfinding",
    "audio",
    "entities",
    "items",
    "renderers",
    "storage",
    "universal",
    "oshi",
    "platform",
    "java",
    "lang",
    "util",
    "nio",
    "io",
    "sun",
    "reflect",
    "zip",
    "jar",
    "jdk",
    "nashorn",
    "scripts",
    "runtime",
    "internal",
    "mods",
    "mod",
    "impl",
    "org",
    "com",
    "cn",
    "cc",
    "jp",
    "core",
    "config",
    "registries",
    "lib",
    "ruby",
    "mc",
    "codec",
    "recipe",
    "channel",
    "embedded",
    "done",
    "netty",
    "network",
    "load",
    "github",
    "handler",
    "content",
    "feature",
    "file",
    "machine",
    "shader",
    "general",
    "helper",
    "init",
    "library",
    "api",
    "integration",
    "engine",
    "preload",
    "preinit",
    "hellominecraft",
    "jackhuang",
    "fml",
    "minecraftforge",
    "forge",
    "cpw",
    "modlauncher",
    "launchwrapper",
    "objectweb",
    "asm",
    "event",
    "eventhandler",
    "handshake",
    "modapi",
    "kcauldron",
    "fabricmc",
    "loader",
    "game",
    "knot",
    "launch",
    "mixin",
];

/// The rule table, ported from HMCL's `CrashReportAnalyzer.Rule`.
///
/// Each entry is `(name, pattern, captured-group-names)`. `\R` in the original
/// is `\r?\n` here.
#[rustfmt::skip]
static RULES_SOURCE: &[(&str, &str, &[&str])] = &[
    ("OPENJ9", r"(Open J9 is not supported|OpenJ9 is incompatible|\.J9VMInternals\.)", &[]),
    ("NEED_JDK11", r"(no such method: sun\.misc\.Unsafe\.defineAnonymousClass\(Class,byte\[\],Object\[\]\)Class/invokeVirtual|java\.lang\.UnsupportedClassVersionError: icyllis/modernui/forge/MixinConnector has been compiled by a more recent version of the Java Runtime \(class file version 55\.0\), this version of the Java Runtime only recognizes class file versions up to 52\.0|java\.lang\.IllegalArgumentException: The requested compatibility level JAVA_11 could not be set\. Level is not supported by the active JRE or ASM version)", &[]),
    ("TOO_OLD_JAVA", r"java\.lang\.UnsupportedClassVersionError: (.*?) version (?<expected>\d+)\.0", &["expected"]),
    ("JVM_32BIT", r"(Could not reserve enough space for (.*?)KB object heap|The specified size exceeds the maximum representable size|Invalid maximum heap size)", &[]),
    ("GL_OPERATION_FAILURE", r"(1282: Invalid operation|Maybe try a lower resolution resourcepack\?)", &[]),
    ("OPENGL_NOT_SUPPORTED", r"The driver does not appear to support OpenGL", &[]),
    ("GRAPHICS_DRIVER", r"(Pixel format not accelerated|GLX: Failed to create context: GLXBadFBConfig|Couldn't set pixel format|net\.minecraftforge\.fml\.client\.SplashProgress|org\.lwjgl\.LWJGLException|EXCEPTION_ACCESS_VIOLATION(.|\n|\r)+# C {2}\[(ig|atio|nvoglv))", &[]),
    ("MACOS_FAILED_TO_FIND_SERVICE_PORT_FOR_DISPLAY", r"java\.lang\.IllegalStateException: GLFW error before init: \[0x10008\]Cocoa: Failed to find service port for display", &[]),
    ("OUT_OF_MEMORY", r"(java\.lang\.OutOfMemoryError|The system is out of physical RAM or swap space|Out of Memory Error|Error occurred during initialization of VM\r?\nToo small maximum heap)", &[]),
    ("MEMORY_EXCEEDED", r"There is insufficient memory for the Java Runtime Environment to continue", &[]),
    ("RESOLUTION_TOO_HIGH", r"Maybe try a (lower resolution|lowerresolution) (resourcepack|texturepack)\?", &[]),
    ("JDK_9", r"java\.lang\.ClassCastException: (java\.base/jdk|class jdk)", &[]),
    ("MAC_JDK_8U261", r"Terminating app due to uncaught exception 'NSInternalInconsistencyException', reason: 'NSWindow drag regions should only be invalidated on the Main Thread!'", &[]),
    ("FILE_CHANGED", r"java\.lang\.SecurityException: SHA1 digest error for (?<file>.*)|signer information does not match signer information of other classes in the same package", &["file"]),
    ("NO_SUCH_METHOD_ERROR", r"java\.lang\.NoSuchMethodError: (?<class>.*?)", &["class"]),
    ("NO_CLASS_DEF_FOUND_ERROR", r"java\.lang\.NoClassDefFoundError: (?<class>.*)", &["class"]),
    ("ILLEGAL_ACCESS_ERROR", r"java\.lang\.IllegalAccessError: tried to access class (.*?) from class (?<class>.*?)", &["class"]),
    ("DUPLICATED_MOD", r"Found a duplicate mod (?<name>.*) at (?<path>.*)", &["name", "path"]),
    ("MOD_RESOLUTION", r"ModResolutionException: (?<reason>(.*)[\n\r]*( - (.*)[\n\r]*)+)", &["reason"]),
    ("FORGEMOD_RESOLUTION", r"Missing or unsupported mandatory dependencies:(?<reason>(.*)[\n\r]*(\t(.*)[\n\r]*)+)", &["reason"]),
    ("FORGE_FOUND_DUPLICATE_MODS", r"Found duplicate mods:(?<reason>(.*)\r?\n*(\t(.*)\r?\n*)+)", &["reason"]),
    ("MOD_RESOLUTION_CONFLICT", r"ModResolutionException: Found conflicting mods: (?<sourcemod>.*) conflicts with (?<destmod>.*)", &["sourcemod", "destmod"]),
    ("MOD_RESOLUTION_MISSING", r"ModResolutionException: Could not find required mod: (?<sourcemod>.*) requires (?<destmod>.*)", &["sourcemod", "destmod"]),
    ("MOD_RESOLUTION_MISSING_MINECRAFT", r"ModResolutionException: Could not find required mod: (?<mod>.*) requires \{minecraft @ (?<version>.*)}", &["mod", "version"]),
    ("MOD_RESOLUTION_COLLECTION", r"ModResolutionException: Could not resolve valid mod collection \(at: (?<sourcemod>.*) requires (?<destmod>.*)\)", &["sourcemod", "destmod"]),
    ("FILE_ALREADY_EXISTS", r"java\.nio\.file\.FileAlreadyExistsException: (?<file>.*)", &["file"]),
    ("LOADING_CRASHED_FORGE", r"LoaderExceptionModCrash: Caught exception from (?<name>.*?) \((?<id>.*)\)", &["name", "id"]),
    ("BOOTSTRAP_FAILED", r"Failed to create mod instance\. ModID: (?<id>.*?),", &["id"]),
    ("LOADING_CRASHED_FABRIC", r"Could not execute entrypoint stage '(.*?)' due to errors, provided by '(?<id>.*)'!", &["id"]),
    ("FABRIC_VERSION_0_12", r"java\.lang\.NoClassDefFoundError: org/spongepowered/asm/mixin/transformer/FabricMixinTransformerProxy", &[]),
    ("MODLAUNCHER_8", r"java\.lang\.NoSuchMethodError: ('void sun\.security\.util\.ManifestEntryVerifier\.<init>\(java\.util\.jar\.Manifest\)'|sun\.security\.util\.ManifestEntryVerifier\.<init>\(Ljava/util/jar/Manifest;\)V)", &[]),
    ("DEBUG_CRASH", r"Manually triggered debug crash", &[]),
    ("CONFIG", r"Failed loading config file (?<file>.*?) of type (.*?) for modid (?<id>.*)", &["id", "file"]),
    ("FABRIC_WARNINGS", r"(Warnings were found!|Incompatible mod set!|Incompatible mods found!)(.*?)[\n\r]+(?<reason>[^\[]+)\[", &["reason"]),
    ("ENTITY", r"Entity Type: (?<type>.*)[\w\W\n\r]*?Entity's Exact location: (?<location>.*)", &["type", "location"]),
    ("BLOCK", r"Block: (?<type>.*)[\w\W\n\r]*?Block location: (?<location>.*)", &["type", "location"]),
    ("UNSATISFIED_LINK_ERROR", r"java\.lang\.UnsatisfiedLinkError: Failed to locate library: (?<name>.*)", &["name"]),
    ("OPTIFINE_IS_NOT_COMPATIBLE_WITH_FORGE", r"(java\.lang\.NoSuchMethodError: 'java\.lang\.Class sun\.misc\.Unsafe\.defineAnonymousClass\(java\.lang\.Class, byte\[\], java\.lang\.Object\[\]\)'|java\.lang\.NoSuchMethodError: 'void net\.minecraft\.client\.renderer\.texture\.SpriteContents\.<init>\(net\.minecraft\.resources\.ResourceLocation, |java\.lang\.NoSuchMethodError: 'void net\.minecraftforge\.client\.gui\.overlay\.ForgeGui\.renderSelectedItemName\(net\.minecraft\.client\.gui\.GuiGraphics, int\)'|java\.lang\.NoSuchMethodError: 'java\.lang\.String com\.mojang\.blaze3d\.systems\.RenderSystem\.getBackendDescription\(\)'|java\.lang\.NoSuchMethodError: 'net\.minecraft\.network\.chat\.FormattedText net\.minecraft\.client\.gui\.Font\.ellipsize\(net\.minecraft\.network\.chat\.FormattedText, int\)'|java\.lang\.NoSuchMethodError: 'void net\.minecraft\.server\.level\.DistanceManager\.(.*?)\(net\.minecraft\.server\.level\.TicketType, net\.minecraft\.world\.level\.ChunkPos, int, java\.lang\.Object, boolean\)'|java\.lang\.NoSuchMethodError: 'void net\.minecraft\.client\.renderer\.block\.model\.BakedQuad\.<init>\(int\[\], int, net\.minecraft\.core\.Direction, net\.minecraft\.client\.renderer\.texture\.TextureAtlasSprite, boolean, boolean\)'|TRANSFORMER/net\.optifine/net\.optifine\.reflect\.Reflector\.<clinit>\(Reflector\.java)", &[]),
    ("MOD_FILES_ARE_DECOMPRESSED", r"(The directories below appear to be extracted jar files\. Fix this before you continue|Extracted mod jars found, loading will NOT continue)", &[]),
    ("OPTIFINE_CAUSES_THE_WORLD_TO_FAIL_TO_LOAD", r"java\.lang\.NoSuchMethodError: net\.minecraft\.world\.server\.ChunkManager$ProxyTicketManager\.shouldForceTicks\(J\)Z", &[]),
    ("TOO_MANY_MODS_LEAD_TO_EXCEEDING_THE_ID_LIMIT", r"maximum id range exceeded", &[]),
    ("MODMIXIN_FAILURE", r"(MixinApplyError|Mixin prepare failed |Mixin apply failed |mixin\.injection\.throwables\.|\.mixins\.json\] FAILED during \))", &[]),
    ("MIXIN_APPLY_MOD_FAILED", r"Mixin apply for mod (?<id>.*) failed", &["id"]),
    ("FORGE_ERROR", r"An exception was thrown, the game will display an error screen and halt\.\r?\n*(?<reason>.*\r?\n*(\s*at .*\r?\n)+)", &["reason"]),
    ("MOD_RESOLUTION0", r"(\tMod File:|-- MOD |\tFailure message:)", &[]),
    ("FORGE_REPEAT_INSTALLATION", r"MultipleArgumentsForOptionException: Found multiple arguments for option (.*?), but you asked for only one", &[]),
    ("OPTIFINE_REPEAT_INSTALLATION", r"ResolutionException: Module optifine reads another module named optifine", &[]),
    ("JAVA_VERSION_IS_TOO_HIGH", r#"(Unable to make protected final java\.lang\.Class java\.lang\.ClassLoader\.defineClass|java\.lang\.NoSuchFieldException: ucp|Unsupported class file major version|because module java\.base does not export|java\.lang\.ClassNotFoundException: jdk\.nashorn\.api\.scripting\.NashornScriptEngineFactory|java\.lang\.ClassNotFoundException: java\.lang\.invoke\.LambdaMetafactory|Exception in thread "main" java\.lang\.NullPointerException: Cannot read the array length because "urls" is null)"#, &[]),
    ("INSTALL_MIXINBOOTSTRAP", r"java\.lang\.ClassNotFoundException: org\.spongepowered\.asm\.launch\.MixinTweaker", &[]),
    ("MOD_NAME", r"Invalid module name: '' is not a Java identifier", &[]),
    ("INCOMPLETE_FORGE_INSTALLATION", r"(java\.io\.UncheckedIOException: java\.io\.IOException: Invalid paths argument, contained no existing paths: \[(.*?)(forge-(.*?)-client\.jar|fmlcore-(.*?)\.jar)\]|Failed to find Minecraft resource version (.*?) at (.*?)forge-(.*?)-client\.jar|Cannot find launch target fmlclient, unable to launch|java\.lang\.IllegalStateException: Could not find net/minecraft/client/Minecraft\.class in classloader SecureModuleClassLoader)", &[]),
    ("NIGHT_CONFIG_FIXES", r"com\.electronwill\.nightconfig\.core\.io\.ParsingException: Not enough data available", &[]),
    ("SHADERS_MOD", r"java\.lang\.RuntimeException: Shaders Mod detected\. Please remove it, OptiFine has built-in support for shaders\.", &[]),
    ("MOD_FOREST_OPTIFINE", r"Error occurred applying transform of coremod META-INF/asm/multipart\.js function render", &[]),
    ("PERFORMANT_FOREST_OPTIFINE", r"org\.spongepowered\.asm\.mixin\.injection\.throwables\.InjectionError: Critical injection failure: Redirector OnisOnLadder\(Lnet/minecraft/block/BlockState;Lnet/minecraft/world/World;Lnet/minecraft/util/math/BlockPos;Lnet/minecraft/entity/LivingEntity;\)Z in performant\.mixins\.json:entity\.LivingEntityMixin failed injection check, \(0/1\) succeeded\. Scanned 1 target\(s\)\. Using refmap performant\.refmap\.json", &[]),
    ("TWILIGHT_FOREST_OPTIFINE", r"java\.lang\.IllegalArgumentException: (.*) outside of image bounds (.*)", &[]),
    ("JADE_FOREST_OPTIFINE", r"Critical injection failure: LVT in net/minecraft/client/renderer/GameRenderer::m_109093_\(FJZ\)V has incompatible changes at opcode 760 in callback jade\.mixins\.json:GameRendererMixin-\>@Inject::jade\$runTick\(FJZLorg/spongepowered/asm/mixin/injection/callback/CallbackInfo;IILcom/mojang/blaze3d/platform/Window;Lorg/joml/Matrix4f;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/gui/GuiGraphics;\)V\.", &[]),
    ("NEOFORGE_FOREST_OPTIFINE", r"cpw\.mods\.modlauncher\.InvalidLauncherSetupException: Invalid Services found OptiFine", &[]),
    ("RTSS_FOREST_SODIUM", r"RivaTuner Statistics Server \(RTSS\) is not compatible with Sodium", &[]),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn reason<'a>(reasons: &'a [Reason], rule: &str) -> Option<&'a Reason> {
        reasons.iter().find(|reason| reason.rule == rule)
    }

    #[test]
    fn every_rule_compiles() {
        // `analyze` drops a pattern that will not compile, which would silently
        // lose a rule; this keeps such a typo from shipping.
        assert_eq!(
            RULES.len(),
            RULES_SOURCE.len(),
            "some crash rules failed to compile"
        );
    }

    #[test]
    fn an_out_of_memory_crash_is_recognised() {
        let log = "java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Main.main(Main.java:1)";
        assert!(reason(&analyze(log), "OUT_OF_MEMORY").is_some());
    }

    #[test]
    fn a_too_old_java_captures_the_expected_version() {
        let log = "java.lang.UnsupportedClassVersionError: foo/Bar has been compiled by a more recent version of the Java Runtime (class file version 61.0)";
        let reasons = analyze(log);
        let reason = reason(&reasons, "TOO_OLD_JAVA").expect("TOO_OLD_JAVA");
        // The `(.*?) version` prefix lets the version land in the capture.
        assert!(
            reason
                .fields
                .first()
                .map(String::as_str)
                .unwrap_or_default()
                .contains("61")
        );
    }

    #[test]
    fn a_crash_report_is_extracted_from_the_console() {
        let log = "before\n---- Minecraft Crash Report ----\nDescription: Test\n\nstack\n#@!@# Game crashed! Crash report saved to: /tmp/x.txt\nafter";
        let report = extract_crash_report(log).expect("a report");
        assert!(report.starts_with("---- Minecraft Crash Report ----"));
        assert!(report.contains("Description: Test"));
    }
}
