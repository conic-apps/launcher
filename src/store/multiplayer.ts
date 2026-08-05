// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import {
    createRoom as createRoomCommand,
    getSessionState,
    joinRoom as joinRoomCommand,
    leaveRoom as leaveRoomCommand,
    on,
    queryPeers,
    toStateName,
    type MultiplayerState,
    type PeerInfo,
    type PlayerProfile,
} from "@conic/multiplayer"
import { defineStore } from "pinia"
import { computed, ref, watch } from "vue"

const NAT_POLL_UNKNOWN_INTERVAL = 10_000
const NAT_POLL_KNOWN_INTERVAL = 60_000

export const useMultiplayerStore = defineStore("multiplayer", () => {
    const state = ref<MultiplayerState>("waiting")
    const roomCode = ref("")
    const players = ref<PlayerProfile[]>([])
    const peers = ref<PeerInfo[]>([])
    const url = ref("")
    const fault = ref<{ code: number; message: string } | null>(null)
    const dialogVisible = ref(false)
    const mode = computed(() =>
        state.value === "host-ok" ? "host" : state.value === "guest-ok" ? "guest" : "unset",
    )

    let refreshing = false
    async function refresh() {
        if (refreshing) return
        refreshing = true
        try {
            const session = await getSessionState()
            state.value = toStateName(session.state)
            roomCode.value = session.room_code
            if ("profiles" in session.detail) {
                players.value = session.detail.profiles
            }
            if ("url" in session.detail) {
                url.value = session.detail.url
            }
            if ("error" in session.detail) {
                const error = session.detail.error
                fault.value = { code: error.code, message: error.message }
            }
            await refreshPeers()
        } catch (error) {
            console.error(error)
        } finally {
            refreshing = false
        }
    }

    let peersPollTimer: ReturnType<typeof setInterval> | null = null

    function stopPeersPolling() {
        if (peersPollTimer !== null) {
            clearInterval(peersPollTimer)
            peersPollTimer = null
        }
    }

    function isNatKnown(peersToCheck: PeerInfo[]): boolean {
        const local = peersToCheck.find((peer) => peer.is_local)
        return local !== undefined && local.nat !== 0
    }

    function schedulePeersPolling() {
        stopPeersPolling()
        if (state.value !== "host-ok" && state.value !== "guest-ok") return
        const interval = isNatKnown(peers.value)
            ? NAT_POLL_KNOWN_INTERVAL
            : NAT_POLL_UNKNOWN_INTERVAL
        peersPollTimer = setInterval(() => {
            refreshPeers()
        }, interval)
    }

    async function refreshPeers() {
        try {
            peers.value = await queryPeers()
        } catch {
            peers.value = []
        }
        schedulePeersPolling()
    }

    let initialized = false
    async function init() {
        if (initialized) return
        initialized = true
        on("state-changed", (payload) => {
            state.value = toStateName(payload.state)
            refresh()
        })
        on("player-joined", () => {
            refresh()
        })
        on("player-left", () => {
            refresh()
        })
        on("host-ready", (payload) => {
            roomCode.value = payload.room
            refresh()
        })
        on("guest-ready", (payload) => {
            url.value = payload.url
            refresh()
        })
        on("fault", (payload) => {
            fault.value = { code: payload.code, message: payload.message }
            state.value = "exception"
        })
        await refresh()
    }

    watch(state, () => {
        schedulePeersPolling()
    })

    async function createRoom(playerName: string) {
        fault.value = null
        await createRoomCommand({ playerName })
    }

    async function joinRoom(code: string, playerName: string) {
        fault.value = null
        await joinRoomCommand({ roomCode: code, playerName })
    }

    async function leaveRoom() {
        await leaveRoomCommand()
        stopPeersPolling()
        state.value = "waiting"
        roomCode.value = ""
        players.value = []
        peers.value = []
        url.value = ""
        fault.value = null
    }

    async function checkNAT(): Promise<PeerInfo[]> {
        return await queryPeers()
    }

    return {
        state,
        roomCode,
        players,
        peers,
        url,
        fault,
        dialogVisible,
        mode,
        init,
        createRoom,
        joinRoom,
        leaveRoom,
        checkNAT,
    }
})
