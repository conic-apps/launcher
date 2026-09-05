// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { computed, onMounted, ref, watch, type Ref } from "vue"
import { getDataLocation, getInstanceRoot } from "@conic/folder"
import { ModInstalledInfo, checkModInstalled, parseMods, removeModFiles } from "@conic/content"
import { ProjectVersion, listProjectVersions } from "@conic/modrinth"
import {
    File as CurseforgeFile,
    GetModFilesParams,
    HashAlgo,
    ModLoaderType as CurseforgeModLoaderType,
    getModFileDownloadUrl,
    getModFiles,
} from "@conic/curseforge"
import { DownloadTask, DownloadTaskInfo, DownloadTaskType } from "@conic/download"
import { useInstanceStore } from "@/store/instance"
import { useFavorites } from "./useFavorites"

export type ContentPlatform = "modrinth" | "curseforge"
export type ContentActionsType = "mod" | "resourcepack" | "pack"

const CURSEFORGE_LOADER_TYPES: Record<string, CurseforgeModLoaderType> = {
    forge: CurseforgeModLoaderType.Forge,
    fabric: CurseforgeModLoaderType.Fabric,
    quilt: CurseforgeModLoaderType.Quilt,
    neoforge: CurseforgeModLoaderType.NeoForge,
}

export function useContentActions(
    platform: ContentPlatform,
    contentType: ContentActionsType,
    projectId: Ref<string | number | null>,
) {
    const instanceStore = useInstanceStore()
    const { loadFavorites, isFavorited, toggleFavorite } = useFavorites()

    const isMod = contentType === "mod"
    const installedInfo = ref(null as null | ModInstalledInfo)
    const checkingInstalled = ref(false)
    const operating = ref(false)

    const installed = computed(() => installedInfo.value?.installed ?? false)
    const installedVersion = computed(() => installedInfo.value?.mods[0]?.version ?? null)

    async function refreshInstalled() {
        const instance = instanceStore.currentInstance
        const id = projectId.value
        if (!isMod || !instance || id === null || id === undefined) {
            installedInfo.value = null
            return
        }
        checkingInstalled.value = true
        installedInfo.value = null
        try {
            installedInfo.value = await checkModInstalled(instance.id, platform, String(id))
        } catch (error) {
            console.error("Failed to check mod installed status:", error)
            installedInfo.value = null
        } finally {
            checkingInstalled.value = false
        }
    }

    watch(
        projectId,
        () => {
            void refreshInstalled()
        },
        { immediate: true },
    )

    async function resolveTargetDir(instanceId: string): Promise<string> {
        if (contentType === "pack") {
            const dataLocation = await getDataLocation()
            return `${dataLocation.root}/modpacks`
        }
        const instanceRoot = await getInstanceRoot(instanceId)
        return contentType === "mod" ? `${instanceRoot}/mods` : `${instanceRoot}/resourcepacks`
    }

    function pickModrinthVersion(
        versions: ProjectVersion[],
        minecraft?: string,
        loader?: string,
    ): ProjectVersion | undefined {
        const compatible = versions.find(
            (version) =>
                version.files.length > 0 &&
                (!minecraft ||
                    !version.game_versions?.length ||
                    version.game_versions.includes(minecraft)) &&
                (!loader || !version.loaders?.length || version.loaders.includes(loader)),
        )
        return compatible ?? versions.find((version) => version.files.length > 0)
    }

    async function resolveDownloadTask(instanceId: string, id: string): Promise<DownloadTask> {
        const runtime = instanceStore.currentInstance?.config.runtime
        const minecraft = runtime?.minecraft
        const loader =
            isMod && runtime?.mod_loader_type ? runtime.mod_loader_type.toLowerCase() : undefined
        const targetDir = await resolveTargetDir(instanceId)

        if (platform === "modrinth") {
            const versions = await listProjectVersions(id, {
                loaders: loader ? JSON.stringify([loader]) : undefined,
                game_versions: minecraft ? JSON.stringify([minecraft]) : undefined,
            })
            const version = pickModrinthVersion(versions, minecraft, loader)
            if (!version) {
                throw new Error(`No compatible Modrinth version found for project ${id}`)
            }
            const file = version.files.find((file) => file.primary) ?? version.files[0]
            if (!file) {
                throw new Error(`No downloadable file found for Modrinth project ${id}`)
            }
            const taskInfo: DownloadTaskInfo = {
                url: file.url,
                file: `${targetDir}/${file.filename}`,
                size_bytes: file.size,
                checksum: file.hashes?.sha512 ? { Sha512: file.hashes.sha512 } : "None",
                task_type: DownloadTaskType.ModrinthMod,
            }
            return new DownloadTask(taskInfo)
        }

        const params: GetModFilesParams = {}
        if (minecraft) params.gameVersion = minecraft
        if (isMod && loader) params.modLoaderType = CURSEFORGE_LOADER_TYPES[loader]

        const response = await getModFiles(Number(id), params)
        const file: CurseforgeFile | undefined =
            response.data.find((file) => file.isAvailable) ?? response.data[0]
        if (!file) {
            throw new Error(`No compatible CurseForge file found for mod ${id}`)
        }
        let url = file.downloadUrl
        if (!url) {
            const urlResponse = await getModFileDownloadUrl(Number(id), file.id)
            url = urlResponse.data
        }
        const sha1 = file.hashes.find((hash) => hash.algo === HashAlgo.Sha1)?.value
        const taskInfo: DownloadTaskInfo = {
            url,
            file: `${targetDir}/${file.fileName}`,
            size_bytes: file.fileLength,
            checksum: sha1 ? { Sha1: sha1 } : "None",
            task_type: DownloadTaskType.CurseforgeMod,
        }
        return new DownloadTask(taskInfo)
    }

    async function install() {
        const instance = instanceStore.currentInstance
        const id = projectId.value
        if (!instance || id === null || id === undefined) return
        operating.value = true
        try {
            const task = await resolveDownloadTask(instance.id, String(id))
            await task.start()
            if (isMod) {
                await parseMods(instance.id)
                await refreshInstalled()
            }
        } catch (error) {
            console.error("Failed to download content:", error)
        } finally {
            operating.value = false
        }
    }

    async function removeMod() {
        const instance = instanceStore.currentInstance
        const info = installedInfo.value
        if (!isMod || !instance || !info?.installed || info.mods.length === 0) return
        operating.value = true
        try {
            await removeModFiles(
                instance.id,
                info.mods.map((mod) => mod.path),
            )
            installedInfo.value = null
        } catch (error) {
            console.error("Failed to remove mod:", error)
        } finally {
            operating.value = false
        }
    }

    onMounted(async () => {
        await loadFavorites()
    })

    return {
        isFavorited,
        toggleFavorite,
        installedInfo,
        installed,
        installedVersion,
        checkingInstalled,
        operating,
        install,
        removeMod,
    }
}
