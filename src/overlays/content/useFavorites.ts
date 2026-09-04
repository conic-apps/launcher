// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { ref } from "vue"
import {
    listFavorites,
    addFavorite as apiAddFavorite,
    removeFavorite as apiRemoveFavorite,
} from "@conic/content"

const favoriteKeys = ref(new Set<string>())
let loaded = false

function makeKey(platform: string, contentType: string, projectId: string): string {
    return `${platform}:${contentType}:${projectId}`
}

export function useFavorites() {
    async function loadFavorites() {
        if (loaded) return
        try {
            const favorites = await listFavorites()
            favoriteKeys.value = new Set(
                favorites.map((f) => makeKey(f.platform, f.content_type, f.project_id)),
            )
            loaded = true
        } catch (error) {
            console.error("Failed to load favorites:", error)
        }
    }

    function isFavorited(platform: string, contentType: string, projectId: string): boolean {
        return favoriteKeys.value.has(makeKey(platform, contentType, projectId))
    }

    async function toggleFavorite(
        platform: string,
        contentType: string,
        projectId: string,
    ): Promise<void> {
        const key = makeKey(platform, contentType, projectId)
        const newKeys = new Set(favoriteKeys.value)
        if (newKeys.has(key)) {
            newKeys.delete(key)
            favoriteKeys.value = newKeys
            try {
                await apiRemoveFavorite(platform, contentType, projectId)
            } catch (error) {
                console.error("Failed to remove favorite:", error)
                newKeys.add(key)
                favoriteKeys.value = newKeys
            }
        } else {
            newKeys.add(key)
            favoriteKeys.value = newKeys
            try {
                await apiAddFavorite(platform, contentType, projectId)
            } catch (error) {
                console.error("Failed to add favorite:", error)
                newKeys.delete(key)
                favoriteKeys.value = newKeys
            }
        }
    }

    return {
        loadFavorites,
        isFavorited,
        toggleFavorite,
        favoriteKeys,
    }
}
