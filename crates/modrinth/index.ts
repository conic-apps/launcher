// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Channel, invoke } from "@tauri-apps/api/core"

export type SearchedProjects = {
    hits: {
        slug?: string
        title?: string
        description?: string
        categories?: string[]
        client_side?: "required" | "optional" | "unsupported" | "unknown"
        server_side?: "required" | "optional" | "unsupported" | "unknown"
        project_type: "mod" | "modpack" | "resourcepack" | "shader"
        downloads: number
        icon_url?: string
        color?: number
        thread_id?: string
        monetization_status?: string
        project_id: string
        author: string
        display_categories: string[]
        versions: string[]
        follows: number
        date_created: string
        latest_version?: string
        license: string
        gallery?: string[]
        featured_gallery?: string
    }[]
    offset: number
    limit: number
    total_hits: number
}

export type SearchParameters = {
    query?: string
    facets?: string
    index?: string
    offset?: number
    limit?: number
}

export async function searchProjects(params: SearchParameters): Promise<SearchedProjects> {
    return await invoke("plugin:modrinth|cmd_search_projects", { params })
}

export type Project = {
    slug?: string
    title?: string
    description?: string
    categories?: string[]
    client_side?: "required" | "optional" | "unsupported" | "unknown"
    server_side?: "required" | "optional" | "unsupported" | "unknown"
    body?: string
    status?:
        | "approved"
        | "archived"
        | "rejected"
        | "draft"
        | "unlisted"
        | "processing"
        | "withheld"
        | "scheduled"
        | "private"
        | "unknown"
    requested_status?: "approved" | "archived" | "unlisted" | "private" | "draft"
    additional_categories?: string[]
    issues_utl?: string
    source_url?: string
    wiki_url?: string
    discord_url?: string
    donation_urls?: {
        id?: string
        platform?: string
        url?: string
    }[]
    project_type: "mod" | "modpack" | "resourcepack" | "shader"
    downloads: number
    icon_url?: string
    color?: number
    thread_id?: string
    monetization_status?: string
    id: string
    team: string
    body_url?: string
    moderator_message?: {
        message?: string
        body?: string
    }
    published: string
    updated: string
    approved?: string
    queued?: string
    followers: number
    license?: {
        id?: string
        name?: string
        url?: string
    }
    versions?: string[]
    game_versions?: string[]
    loaders?: string[]
    gallery?: {
        url: string
        featured: boolean
        title?: string
        description?: string
        created: string
        ordering: number
    }
}

export async function getProject(idOrSlug: string): Promise<Project> {
    return await invoke("plugin:modrinth|cmd_get_project", { idOrSlug })
}

export async function getMultipleProject(ids: string[]): Promise<Project[]> {
    return await invoke("plugin:modrinth|cmd_get_multiple_projects", { ids })
}

export type ProjectVersion = {
    name?: string
    version_number?: string
    changelog?: string
    dependencies: {
        version_id?: string
        project_id?: string
        file_name?: string
        dependency_type: "required" | "optional" | "incompatible" | "embedded"
    }
    game_versions?: string[]
    version_type?: "release" | "beta" | "alpha"
    loaders?: string[]
    featured?: boolean
    status?: "listed" | "archived" | "draft" | "unlisted" | "scheduled" | "unknown"
    requested_status?: "listed" | "archived" | "draft" | "unlisted"
    id: string
    project_id: string
    author_id: string
    date_published: string
    downloads: number
    changelog_url: string
    files: {
        hashes: {
            sha512: string
            sha1: string
        }
        url: string
        filename: string
        primary: boolean
        size: number
        file_type:
            | "required-resource-pack"
            | "optional-resource-pack"
            | "sources-jar"
            | "dev-jar"
            | "javadoc-jar"
            | "unknown"
            | "signature"
    }[]
}

export type Dependencies = {
    projects: Project[]
    versions: ProjectVersion[]
}
export async function getAllDependencies(id: string): Promise<Dependencies> {
    return await invoke("plugin:modrinth|cmd_get_all_dependencies", { id })
}

export type ListProjectVersionsParams = {
    loaders?: string
    game_versions?: string
    featured?: boolean
    include_changelog?: boolean
}

export async function listProjectVersions(
    idOrSlug: string,
    params: ListProjectVersionsParams,
): Promise<ProjectVersion[]> {
    return await invoke("plugin:modrinth|cmd_list_project_versions", { idOrSlug, params })
}

export type DownloadProgress = {
    completed: number
    total: number
    step: "VerifyExistingFiles" | "DownloadFiles"
    speed: number
}

export enum DownloadErrorKind {
    Network = "Network",
    Io = "Io",
    ChecksumMissmatch = "ChecksumMissmatch",
    UrlParse = "UrlParse",
    ChunkLengthMismatch = "ChunkLengthMismatch",
    Aborted = "Aborted",
}

export type ModFile = {
    url: string
    file_name: string
    sha512: string
    size_bytes: number
}

export class InstallTask {
    protected _instanceId: `${string}-${string}-${string}-${string}-${string}`
    protected _taskId: `${string}-${string}-${string}-${string}-${string}`
    protected _modFile: ModFile
    protected _callbacks?: {
        onStart?: () => void
        onProgress?: (task: DownloadProgress) => void
        onFailed?: (error: { kind: DownloadErrorKind; message: string }) => void
        onSucceed?: () => void
        onCancelled?: () => void
    }
    constructor(
        instanceId: `${string}-${string}-${string}-${string}-${string}`,
        modFile: ModFile,
        callbacks?: typeof this._callbacks,
    ) {
        const taskId = crypto.randomUUID()
        this._instanceId = instanceId
        this._taskId = taskId
        this._modFile = modFile
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<{ task_id: string; progress: DownloadProgress }>()
        channel.onmessage = (message) => {
            if (message.task_id === this._taskId) {
                this._callbacks?.onProgress?.(message.progress)
            }
        }
        try {
            await invoke("plugin:modrinth|cmd_create_download_mod_task", {
                modFile: this._modFile,
                taskId: this._taskId,
                instanceId: this._instanceId,
                channel,
            })
            this._callbacks?.onSucceed?.()
        } catch (error: any) {
            if (error.kind && error.message) {
                const kind = error.kind as DownloadErrorKind
                if (kind === DownloadErrorKind.Aborted) {
                    this._callbacks?.onCancelled?.()
                } else {
                    this._callbacks?.onFailed?.(error)
                }
            } else {
                throw error
            }
        }
    }
    async cancel() {
        await invoke("plugin:modrinth|cmd_cancel_download_task", { taskId: this._taskId })
    }
}
