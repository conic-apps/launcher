// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type JavaVendor =
    | "oracle"
    | "openjdk"
    | "eclipse_adoptium"
    | "microsoft"
    | "amazon_corretto"
    | "azul_zulu"
    | "bellsoft_liberica"
    | "semeru"
    | "sap"
    | "dragonwell"
    | "unknown"

export type JavaArch = "x64" | "x86" | "aarch64" | "arm" | "unknown"

export type JavaRuntime = {
    path: string
    java_home?: string
    major_version: number
    version: string
    vendor: JavaVendor
    arch: JavaArch
    is_jdk: boolean
    is_managed: boolean
    is_valid: boolean
}

export type JavaVersionGroup = {
    major_version: number
    runtimes: JavaRuntime[]
}

export type JavaScanResult = {
    runtimes: JavaRuntime[]
    groups: JavaVersionGroup[]
}

export async function scanJava(): Promise<JavaScanResult> {
    return await invoke("plugin:java-runtime|cmd_scan_java")
}
