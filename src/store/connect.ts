// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { isLibraryValid } from "@conic/terracotta"
import { defineStore } from "pinia"

export const useDialogStore = defineStore("terracotta", {
    state: () => ({
        isLibraryValid: false,
    }),
    actions: {
        async checkLibraryValid() {
            try {
                await isLibraryValid()
                this.isLibraryValid = true
            } catch {
                this.isLibraryValid = false
            }
        },
    },
})
