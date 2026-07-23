import { defineStore } from "pinia"

export type Page = "game" | "settings" | "market" | "accounts"

export const useNavigationStore = defineStore("navigation", {
    state: () => ({
        currentPage: "game" as Page,
        history: [] as Page[],
    }),
    actions: {
        navigate(page: Page) {
            if (this.currentPage !== page) {
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
