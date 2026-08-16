import { defineStore } from "pinia"
import { useInstanceStore } from "./instance"
import { ref, watch } from "vue"
import {
    getAllLevels,
    getAllResourcepacks,
    Level,
    listScreenshots,
    Mod,
    parseMods,
    Resourcepack,
} from "@conic/content"

type GameContentCacheValue = {
    [K in keyof GameContent]: { data: GameContent[K] | null; time: number }
}

const gameContentCache = {} as Record<string, GameContentCacheValue>

/// For current instance only
export type GameContent = {
    saves: Record<string, Level> | null
    mods: Mod[] | null
    resourcepacks: Resourcepack[] | null
    screenshots: string[] | null
}

const CACHE_TTL = 60 * 1000
const REFRESH_COOL_DOWN = 10 * 1000

function getValidContentFromCacheEntries(instanceId: string): GameContent {
    const cache = gameContentCache[instanceId]
    if (!cache) return { saves: null, mods: null, resourcepacks: null, screenshots: null }

    return Object.fromEntries(
        Object.entries(cache).map(([key, item]) => {
            const isValid = Date.now() - item.time < CACHE_TTL
            return [key, isValid ? item.data : null]
        }),
    ) as GameContent
}

export const useGameContentStore = defineStore("gameContent", () => {
    const gameContent = ref<GameContent>({
        saves: null,
        mods: null,
        resourcepacks: null,
        screenshots: null,
    })
    const instanceStore = useInstanceStore()

    function createContentLoader<K extends keyof GameContent>(
        key: K,
        instanceId: string,
        fetcher: () => Promise<GameContent[K]>,
        hasValidCache: boolean,
    ) {
        return async () => {
            if (gameContent.value[key] !== null) {
                if (!hasValidCache) {
                } else {
                    const lastRefreshTime = gameContentCache[instanceId]?.[key]?.time || 0
                    const isCoolingDown = Date.now() - lastRefreshTime < REFRESH_COOL_DOWN

                    if (isCoolingDown) return
                }
            }

            const data = await fetcher()

            gameContentCache[instanceId] ??= {} as GameContentCacheValue
            gameContentCache[instanceId][key] ??= { data: null, time: 0 }

            if (instanceId === instanceStore.currentInstance.id) {
                gameContent.value[key] = data
            }

            gameContentCache[instanceId][key].data = data
            gameContentCache[instanceId][key].time = Date.now()
        }
    }

    watch(
        () => instanceStore.currentInstance,
        async (instance) => {
            ;(Object.keys(gameContent.value) as Array<keyof GameContent>).forEach((key) => {
                gameContent.value[key] = null
            })

            const cachedData = getValidContentFromCacheEntries(instance.id)

            Object.assign(gameContent.value, cachedData)

            await Promise.allSettled([
                createContentLoader(
                    "saves",
                    instance.id,
                    () => getAllLevels(instance.id),
                    cachedData.saves !== null,
                )(),
                createContentLoader(
                    "mods",
                    instance.id,
                    async () => {
                        const rawMods = await parseMods(instance.id)
                        return rawMods.filter((mod) => !mod.embedded)
                    },
                    cachedData.mods !== null,
                )(),
                createContentLoader(
                    "resourcepacks",
                    instance.id,
                    () => getAllResourcepacks(instance.id),
                    cachedData.resourcepacks !== null,
                )(),
                createContentLoader(
                    "screenshots",
                    instance.id,
                    () => listScreenshots(instance.id),
                    cachedData.screenshots !== null,
                )(),
            ])
        },
        { immediate: true },
    )

    watch(
        () => instanceStore.instances,
        () => {
            for (const key in gameContentCache) {
                delete gameContentCache[key]
            }
        },
    )

    async function refreshSaves() {
        const instance = instanceStore.currentInstance
        const saves = await getAllLevels(instance.id)

        gameContent.value.saves = saves

        gameContentCache[instance.id] ??= {} as GameContentCacheValue
        gameContentCache[instance.id].saves = {
            data: saves,
            time: Date.now(),
        }
    }

    return { gameContent, refreshSaves }
})
