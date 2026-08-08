// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

/** Minecraft's game id in the CurseForge API. */
export const MINECRAFT_GAME_ID = 432

export enum ModLoaderType {
    Any = 0,
    Forge = 1,
    Cauldron = 2,
    LiteLoader = 3,
    Fabric = 4,
    Quilt = 5,
    NeoForge = 6,
}

export enum ModsSearchSortField {
    Featured = 1,
    Popularity = 2,
    LastUpdated = 3,
    Name = 4,
    Author = 5,
    TotalDownloads = 6,
    Category = 7,
    GameVersion = 8,
    EarlyAccess = 9,
    FeaturedReleased = 10,
    ReleasedDate = 11,
    Rating = 12,
}

export type SortOrder = "asc" | "desc"

export enum FileRelationType {
    EmbeddedLibrary = 1,
    OptionalDependency = 2,
    RequiredDependency = 3,
    Tool = 4,
    Incompatible = 5,
    Include = 6,
}

export enum FileReleaseType {
    Release = 1,
    Beta = 2,
    Alpha = 3,
}

export enum HashAlgo {
    Sha1 = 1,
    Md5 = 2,
}

export type Pagination = {
    index: number
    pageSize: number
    resultCount: number
    totalCount: number
}

/** Every CurseForge response wraps its payload in `data`. The MCIM mirror additionally adds `sync_at`. */
export type ApiResponse<T> = {
    data: T
    pagination?: Pagination
    sync_at?: string
}

export type Category = {
    id: number
    gameId: number
    name: string
    slug: string
    url: string
    iconUrl: string
    dateModified: string
    isClass?: boolean | null
    classId?: number | null
    parentCategoryId?: number | null
    displayIndex?: number | null
}

export type ModAsset = {
    id: number
    modId: number
    title: string
    description: string
    thumbnailUrl: string
    url: string
}

export type ModAuthor = {
    id: number
    name: string
    url: string
}

export type ModLinks = {
    websiteUrl: string
    wikiUrl?: string
    issuesUrl?: string | null
    sourceUrl?: string | null
}

export type FileHash = {
    value: string
    algo: HashAlgo
}

export type FileIndex = {
    gameVersion: string
    fileId: number
    filename: string
    releaseType: FileReleaseType
    gameVersionTypeId?: number | null
}

export type SortableGameVersion = {
    gameVersionName: string
    gameVersionPadded: string
    gameVersion: string
    gameVersionReleaseDate?: string
    gameVersionTypeId?: number | null
}

export type FileDependency = {
    modId: number
    relationType: FileRelationType
}

export type FileModule = {
    name: string
    fingerprint: number
}

export type File = {
    id: number
    gameId: number
    modId: number
    isAvailable: boolean
    displayName: string
    fileName: string
    releaseType: FileReleaseType
    fileStatus: number
    hashes: FileHash[]
    fileDate: string
    fileLength: number
    downloadCount: number
    fileSizeOnDisk?: number | null
    downloadUrl: string
    gameVersions: string[]
    sortableGameVersions: SortableGameVersion[]
    dependencies: FileDependency[]
    exposeAsAlternative?: boolean | null
    parentProjectFileId?: number | null
    alternateFileId?: number | null
    isServerPack?: boolean | null
    serverPackFileId?: number | null
    isEarlyAccessContent?: boolean | null
    earlyAccessEndDate?: string | null
    fileFingerprint: number
    modules: FileModule[]
}

export type Mod = {
    id: number
    gameId: number
    name: string
    slug: string
    links: ModLinks
    summary: string
    status: number
    downloadCount: number
    isFeatured: boolean
    primaryCategoryId: number
    categories: Category[]
    classId?: number | null
    authors: ModAuthor[]
    logo: ModAsset
    screenshots: ModAsset[]
    mainFileId: number
    latestFiles: File[]
    latestFilesIndexes: FileIndex[]
    latestEarlyAccessFilesIndexes: FileIndex[]
    dateCreated: string
    dateModified: string
    dateReleased: string
    allowModDistribution?: boolean | null
    gamePopularityRank: number
    isAvailable: boolean
    thumbsUpCount: number
    rating?: number | null
}

export type SearchModsParams = {
    gameId?: number
    classId?: number
    categoryId?: number
    /** List of category ids, overrides `categoryId`. Pass as a string, e.g. "[1,2,3]". */
    categoryIds?: string
    gameVersion?: string
    /** List of game version strings, overrides `gameVersion`. Pass as a string, e.g. "[\"1.19.1\",\"1.20.1\"]". */
    gameVersions?: string
    searchFilter?: string
    sortField?: ModsSearchSortField
    sortOrder?: SortOrder
    /** Must be coupled with `gameVersion`. */
    modLoaderType?: ModLoaderType
    /** List of mod loader types, overrides `modLoaderType`. Pass as a string, e.g. "[4,5]". */
    modLoaderTypes?: string
    gameVersionTypeId?: number
    authorId?: number
    primaryAuthorId?: number
    slug?: string
    index?: number
    pageSize?: number
}

/** `gameId` defaults to Minecraft (432). */
export async function searchMods(params: SearchModsParams = {}): Promise<ApiResponse<Mod[]>> {
    return await invoke("plugin:curseforge|cmd_search_mods", {
        params: { gameId: MINECRAFT_GAME_ID, ...params },
    })
}

export async function getMod(modId: number): Promise<ApiResponse<Mod>> {
    return await invoke("plugin:curseforge|cmd_get_mod", { modId })
}

export type GetModsBody = {
    /** Mod ids, all must belong to the same game. */
    modIds: number[]
    filterPcOnly?: boolean | null
}

export async function getMods(body: GetModsBody): Promise<ApiResponse<Mod[]>> {
    return await invoke("plugin:curseforge|cmd_get_mods", { body })
}

export type FeaturedModsResponse = {
    featured: Mod[]
    popular: Mod[]
    recentlyUpdated: Mod[]
}

export type GetFeaturedModsBody = {
    gameId?: number
    excludedModIds: number[]
    gameVersionTypeId?: number | null
}

export async function getFeaturedMods(
    body: GetFeaturedModsBody = { excludedModIds: [] },
): Promise<ApiResponse<FeaturedModsResponse>> {
    return await invoke("plugin:curseforge|cmd_get_featured_mods", {
        body: { gameId: MINECRAFT_GAME_ID, ...body },
    })
}

export type GetModDescriptionParams = {
    raw?: boolean
    stripped?: boolean
    markup?: boolean
}

export async function getModDescription(
    modId: number,
    params: GetModDescriptionParams = {},
): Promise<ApiResponse<string>> {
    return await invoke("plugin:curseforge|cmd_get_mod_description", { modId, params })
}

export type GetModFilesParams = {
    gameVersion?: string
    modLoaderType?: ModLoaderType
    gameVersionTypeId?: number
    index?: number
    pageSize?: number
}

export async function getModFiles(
    modId: number,
    params: GetModFilesParams = {},
): Promise<ApiResponse<File[]>> {
    return await invoke("plugin:curseforge|cmd_get_mod_files", { modId, params })
}

export async function getModFile(modId: number, fileId: number): Promise<ApiResponse<File>> {
    return await invoke("plugin:curseforge|cmd_get_mod_file", { modId, fileId })
}

export type GetFilesBody = {
    fileIds: number[]
}

export async function getFiles(body: GetFilesBody): Promise<ApiResponse<File[]>> {
    return await invoke("plugin:curseforge|cmd_get_files", { body })
}

export async function getModFileChangelog(
    modId: number,
    fileId: number,
): Promise<ApiResponse<string>> {
    return await invoke("plugin:curseforge|cmd_get_mod_file_changelog", { modId, fileId })
}

export async function getModFileDownloadUrl(
    modId: number,
    fileId: number,
): Promise<ApiResponse<string>> {
    return await invoke("plugin:curseforge|cmd_get_mod_file_download_url", { modId, fileId })
}
