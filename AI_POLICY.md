# AI Policy

We support using AI (i.e., LLMs) as tools for coding. However, you remain
responsible for any code you publish and we are responsible for any code we
merge and release. We hold a high bar for all contributions to our projects.

Conic Launcher is a launcher, not a website: it holds Microsoft account
credentials, spawns the JVM with arguments you control, listens on a loopback
port for the browser login callback, and ships signed packages for three
platforms. Code in this repository is trusted to run on a user's machine with
their permissions, so the bar below is not a formality.

**AI should not be used to generate comments when communicating with
maintainers**. We expect comments on our projects to be written by humans. We
may hide any comments that we believe are AI generated.

If you are opening an issue, we expect you to describe the problem in your own
words.

If you are opening a pull request, we expect you to be able to explain the
proposed changes in your own words. This includes the pull request body and
responses to questions. **Do not copy responses from the AI when replying to
questions from maintainers.**

If you wish to include context from an interaction with AI in your comments, it
must be in a quote block (e.g., using `>`) and disclosed as such. It must be
accompanied by human commentary explaining the relevance and implications of the
context. Do not share long snippets.

We understand that AI is useful when communicating as a non-native English
speaker. If you are using AI to edit your comments for this purpose, please
take the time to ensure it reflects your own voice and ideas. If using AI for
translation, we recommend writing in your native language and including the AI
translation in a quote block.

This project ships in twelve languages — twelve `.po` catalogues and twelve
localized READMEs under `docs/readme/`. Machine translation is welcome there,
and machine-generated comments are not: a locale's users are being asked to
trust text nobody wrote, and a mistranslated string is indistinguishable from a
broken feature.

This policy was inspired by [uv's AI policy].

[uv's AI policy]: https://github.com/astral-sh/.github/blob/c5187e200db51bfe11d56e13053d29bd3793fdd8/AI_POLICY.md
