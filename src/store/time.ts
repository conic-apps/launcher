import { defineStore } from "pinia"

export const useTimeStore = defineStore("current_time", {
    state: () => {
        return {
            now: Math.round(new Date().getTime() / 1000),
        }
    },
})
