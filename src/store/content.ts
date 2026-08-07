import { defineStore } from "pinia"
import { useInstanceStore } from "./instance"
import { ref, watch } from "vue"
import { getAllLevels, getAllResourcepacks, Level, Resourcepack } from "@conic/content"

let gameContentCache = {} as Record<string, [GameContent, number]>

export type GameContent = {
    saves: Record<string, Level> | null
    resourcepacks: Resourcepack[] | null
}

export const useGameContentStore = defineStore("gameContent", () => {
    const gameContent = ref({ saves: null } as GameContent)
    const instanceStore = useInstanceStore()
    watch(
        () => instanceStore.currentInstance,
        async (instance) => {
            ;(Object.keys(gameContent.value) as Array<keyof GameContent>).forEach((key) => {
                gameContent.value[key] = null
            })
            if (
                gameContentCache[instance.id] &&
                gameContentCache[instance.id][1] - Date.now() < 60 * 1000
            ) {
                gameContent.value = gameContentCache[instance.id][0]
            }
            const loadSaves = async () => {
                gameContent.value.saves = await getAllLevels(instance.id)
            }
            const loadResourcepacks = async () => {
                gameContent.value.resourcepacks = await getAllResourcepacks(instance.id)
            }
            const results = await Promise.allSettled([loadSaves(), loadResourcepacks()])
            if (!results.find((result) => result.status === "rejected")) {
                const timestamp = Date.now()
                gameContentCache[instance.id] = [gameContent.value, timestamp]
            }
        },
        { immediate: true },
    )
    watch(
        () => instanceStore.instances,
        () => (gameContentCache = {}),
    )
    return { gameContent }
})
