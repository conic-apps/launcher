import { defineStore } from "pinia"
import { useInstanceStore } from "./instance"
import { ref, watch } from "vue"
import {
    getAllLevels,
    getAllResourcepacks,
    Level,
    listScreenshots,
    Mod,
    Resourcepack,
} from "@conic/content"

let gameContentCache = {} as Record<string, [GameContent, number]>

export type GameContent = {
    saves: Record<string, Level> | null
    mods: Mod[] | null
    resourcepacks: Resourcepack[] | null
    screenshots: string[] | null
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
            // const loadMods = async () => {
            //     gameContent.value.mods = await parseMods(instance.id)
            // }
            const loadResourcepacks = async () => {
                gameContent.value.resourcepacks = await getAllResourcepacks(instance.id)
            }
            const loadScreenshots = async () => {
                gameContent.value.screenshots = await listScreenshots(instance.id)
            }
            const results = await Promise.allSettled([
                loadSaves(),
                loadResourcepacks(),
                loadScreenshots(),
            ])
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
    async function refreshSaves() {
        const instance = instanceStore.currentInstance
        gameContent.value.saves = await getAllLevels(instance.id)
        if (gameContentCache[instance.id]) {
            gameContentCache[instance.id] = [gameContent.value, Date.now()]
        }
    }
    return { gameContent, refreshSaves }
})
