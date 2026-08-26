// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { getModTranslations } from "@conic/curseforge"
import { getProjectTranslations } from "@conic/modrinth"
import { computed, reactive } from "vue"
import { useI18n } from "vue-i18n"

const modrinthCache = reactive(new Map<string, string>())
const curseforgeCache = reactive(new Map<number, string>())

export function useDescriptionTranslation() {
    const i18n = useI18n()

    const chineseEnabled = computed(() => i18n.locale.value.startsWith("zh"))

    async function translateModrinthDescriptions(projectIds: string[]) {
        if (!chineseEnabled.value) return
        const missing = [...new Set(projectIds)].filter((id) => !modrinthCache.has(id))
        if (missing.length === 0) return
        const translations = await getProjectTranslations(missing)
        for (const { project_id: projectId, translated } of translations) {
            if (translated) modrinthCache.set(projectId, translated)
        }
    }

    async function translateCurseforgeSummaries(modIds: number[]) {
        if (!chineseEnabled.value) return
        const missing = [...new Set(modIds)].filter((id) => !curseforgeCache.has(id))
        if (missing.length === 0) return
        const translations = await getModTranslations(missing)
        for (const { modid, translated } of translations) {
            if (translated) curseforgeCache.set(modid, translated)
        }
    }

    return {
        modrinthCache,
        curseforgeCache,
        translateModrinthDescriptions,
        translateCurseforgeSummaries,
    }
}
