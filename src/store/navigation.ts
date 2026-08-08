// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"

export type Page = "game" | "launch" | "settings" | "market" | "accounts"

export const useNavigationStore = defineStore("navigation", {
    state: () => ({
        currentPage: "game" as Page,
        history: [] as Page[],
    }),
    actions: {
        navigate(page: Page) {
            if (this.currentPage !== page && this.currentPage != "launch") {
                this.history.push(this.currentPage)
            }

            this.currentPage = page
        },

        back() {
            const previous = this.history.pop()

            if (previous) {
                this.currentPage = previous
            }
        },
    },
})
